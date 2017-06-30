#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate structopt_derive;

extern crate futures;
extern crate hyper;
extern crate olifants;
extern crate prometheus;
extern crate structopt;
extern crate tokio_core;

mod error;
mod args;
mod metrics;

use error::*;
use futures::{Future, Stream};
use hyper::server::Http;
use metrics::Metrics;
use olifants::Client;
use std::rc::Rc;
use structopt::StructOpt;
use tokio_core::reactor::Core;

const CRATE_NAME: &'static str = env!("CARGO_PKG_NAME");

quick_main!(|| -> Result<()> {
    let opt = args::Opt::from_args();

    let metrics = Rc::new(Metrics::create(CRATE_NAME, &opt.instance_url).chain_err(
        || "metrics initialization failed",
    )?);

    let mut core = Core::new().chain_err(|| "could not create Core")?;

    let handle = core.handle();
    let listener = {
        let addr = format!("{}:{}", opt.bind, opt.port).parse().chain_err(
            || "invalid address",
        )?;

        tokio_core::net::TcpListener::bind(&addr, &handle)
            .chain_err(|| "failed to bind TCP listener")?
    };

    let client = Client::new(&core.handle(), CRATE_NAME).chain_err(
        || "could not create Client",
    )?;

    let server_metrics = metrics.clone();
    let server = listener
        .incoming()
        .for_each(move |(sock, addr)| {
            Http::new().bind_connection(&handle, sock, addr, server_metrics.clone());
            Ok(())
        })
        .map_err(|e| {
            println!("Server error: {}", e);
        });

    let timeline = client
        .timeline(
            &opt.instance_url,
            opt.access_token,
            olifants::timeline::Endpoint::Local,
        )
        .map_err(|e| Error::with_chain(e, "client error"))
        .for_each(move |event| {
            use olifants::timeline::Event::*;

            if let Update(status) = event {
                metrics.inc(status.account.acct)?;
            }

            Ok(())
        });

    core.handle().spawn(server);
    core.run(timeline).chain_err(|| "timeline failed")
});

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

use args::Args;
use error::*;
use futures::{Future, Stream};
use hyper::server::Http;
use metrics::Metrics;
use olifants::Client;
use std::rc::Rc;
use tokio_core::reactor::Core;

const CRATE_NAME: &'static str = env!("CARGO_PKG_NAME");

quick_main!(|| -> Result<()> {
    let args = Args::init()?;

    let metrics = Rc::new(Metrics::create(CRATE_NAME, &args.instance_url).chain_err(
        || "metrics initialization failed",
    )?);

    let mut core = Core::new().chain_err(|| "could not create Core")?;
    let handle = core.handle();

    let client = Client::new(&handle, CRATE_NAME).chain_err(
        || "could not create Client",
    )?;

    let listener = tokio_core::net::TcpListener::bind(&args.bind_address, &handle)
        .chain_err(|| "failed to bind TCP listener")?;

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
            &args.instance_url,
            args.access_token,
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

    println!("Connecting to stream at {}", args.instance_url);
    println!(
        "Prometheus metrics reporter listening on {}",
        args.bind_address
    );

    core.handle().spawn(server);
    core.run(timeline).chain_err(|| "timeline failed")
});

#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate structopt_derive;

extern crate futures;
extern crate olifants;
extern crate prometheus;
extern crate structopt;
extern crate tokio_core;

mod error;
mod args;
mod metrics;

use error::*;
use futures::Stream;
use metrics::Metrics;
use olifants::Client;
use structopt::StructOpt;
use tokio_core::reactor::Core;

const CRATE_NAME: &'static str = env!("CARGO_PKG_NAME");

quick_main!(|| -> Result<()> {
    let opt = args::Opt::from_args();

    let metrics = Metrics::create(CRATE_NAME, &opt.instance_url).chain_err(
        || "metrics initialization failed",
    )?;

    let mut core = Core::new().chain_err(|| "could not create Core")?;
    let client = Client::new(&core.handle(), CRATE_NAME).chain_err(
        || "could not create Client",
    )?;

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
                println!("{}", metrics.encode_string()?); // TODO: Remove
            }

            Ok(())
        });

    core.run(timeline).chain_err(|| "timeline failed")
});

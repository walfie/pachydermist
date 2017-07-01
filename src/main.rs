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

    let short_instance_url = args.instance_url
        .trim_left_matches("https://")
        .trim_left_matches("http://")
        .to_string();

    let metrics = Rc::new(Metrics::create(&args.namespace, short_instance_url)
        .chain_err(|| "metrics initialization failed")?);

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
            // TODO: stderr
            println!("Server error: {}", e);
        });

    println!("Connecting to stream at {}", args.instance_url);
    println!(
        "Prometheus metrics reporter listening on {}",
        args.bind_address
    );

    core.handle().spawn(server);

    // If the connection is dropped or an error occurs, wait and retry
    loop {
        let metrics_ref = metrics.clone();

        let timeline = client
            .timeline(
                &args.instance_url,
                args.access_token.clone(),
                args.endpoint.clone(),
            )
            .map_err(|e| Error::with_chain(e, "client error"))
            .for_each(move |event| {
                use olifants::timeline::Event::*;

                if let Update(status) = event {
                    metrics_ref.set(
                        &status.account.acct,
                        status.account.statuses_count as f64,
                    )?;
                }

                Ok(())
            });

        if let Err(e) = core.run(timeline) {
            // TODO: stderr
            println!(
                "Encountered error:\n{}",
                error_chain::ChainedError::display(&e)
            );
        }

        // TODO: Exponential backoff
        let delay = ::std::time::Duration::from_secs(5);
        println!("Retrying in 5 seconds...");
        std::thread::sleep(delay);
    }

    // This needs to be here to satisfy the return type, even though it's unreachable
    #[allow(unreachable_code)] Ok(())
});

use error::*;
use futures::{self, IntoFuture};
use hyper;
use hyper::header::ContentLength;
use hyper::server::{Request, Response, Service};
use prometheus::{self, CounterVec, Encoder, Registry, TextEncoder};

pub struct Metrics {
    registry: Registry,
    encoder: TextEncoder,
    counters: CounterVec,
    default_instance: String,
}

impl Metrics {
    pub fn create(namespace: &str, default_instance: String) -> Result<Self> {
        let counter_opts = prometheus::Opts::new("statuses_total", "Number of statuses posted")
            .namespace(namespace)
            .variable_label("instance")
            .variable_label("username");

        let counters = CounterVec::new(counter_opts, &["instance", "username"])
            .chain_err(|| "failed to create Counter")?;

        let registry = Registry::new();
        registry.register(Box::new(counters.clone())).unwrap();
        let encoder = TextEncoder::new();

        Ok(Metrics {
            registry,
            encoder,
            counters,
            default_instance,
        })
    }

    pub fn inc(&self, username: &str) -> Result<()> {
        let mut parts = username.splitn(2, '@');

        let (user, instance): (&str, &str) = match (parts.next(), parts.next()) {
            (Some(user), None) => (user, &self.default_instance),
            (Some(user), Some(instance)) => (user, instance),
            other => {
                // This should theoretically never happen
                bail!(format!(
                    "unexpected state when splitting username {}: {:?}",
                    username,
                    other
                ))
            }
        };

        Ok(
            self.counters
                .get_metric_with_label_values(&[instance, user])
                .chain_err(|| format!("failed to get metric for {}", username))?
                .inc(),
        )
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        let mut buffer = Vec::new();

        self.encoder
            .encode(&self.registry.gather(), &mut buffer)
            .chain_err(|| "failed to encode metrics")?;

        Ok(buffer)
    }
}

impl Service for Metrics {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;

    type Future = futures::future::FutureResult<Self::Response, Self::Error>;

    fn call(&self, _req: Request) -> Self::Future {
        self.encode()
            .map(|body| {
                Response::new()
                    .with_header(ContentLength(body.len() as u64))
                    .with_body(body)
            })
            .or_else(|e| {
                let body = format!("{}", e);

                let resp = Response::new()
                    .with_status(hyper::StatusCode::InternalServerError)
                    .with_header(ContentLength(body.len() as u64))
                    .with_body(body);

                Ok(resp)
            })
            .into_future()
    }
}

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
}

impl Metrics {
    pub fn create(namespace: &str, instance: &str) -> Result<Self> {
        let counter_opts = prometheus::Opts::new("statuses_total", "Number of statuses posted")
            .namespace(namespace)
            .const_label("instance", instance)
            .variable_label("username");

        let counters = CounterVec::new(counter_opts, &["username"]).chain_err(
            || "failed to create Counter",
        )?;

        let registry = Registry::new();
        registry.register(Box::new(counters.clone())).unwrap();
        let encoder = TextEncoder::new();

        Ok(Metrics {
            registry,
            encoder,
            counters,
        })
    }

    pub fn inc<S: AsRef<str>>(&self, username: S) -> Result<()> {
        Ok(
            self.counters
                .get_metric_with_label_values(&[username.as_ref()])
                .chain_err(|| format!("failed to get metric for {}", username.as_ref()))?
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

    pub fn encode_string(&self) -> Result<String> {
        self.encode().and_then(|bytes| {
            String::from_utf8(bytes).chain_err(|| "invalid UTF8")
        })
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

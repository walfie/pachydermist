use error::*;
use futures::{self, IntoFuture};
use hyper;
use hyper::header::ContentLength;
use hyper::server::{Request, Response, Service};
use prometheus::{self, Encoder, GaugeVec, Registry, TextEncoder};

pub struct Metrics {
    registry: Registry,
    encoder: TextEncoder,
    gauges: GaugeVec,
    default_domain: String,
}

impl Metrics {
    pub fn create(namespace: &str, default_domain: String) -> Result<Self> {
        let gauge_opts = prometheus::Opts::new("statuses_total", "Number of statuses posted")
            .namespace(namespace)
            .variable_label("domain")
            .variable_label("user");

        let gauges = GaugeVec::new(gauge_opts, &["domain", "user"]).chain_err(
            || "failed to create Gauge",
        )?;

        let registry = Registry::new();
        registry.register(Box::new(gauges.clone())).unwrap();
        let encoder = TextEncoder::new();

        Ok(Metrics {
            registry,
            encoder,
            gauges,
            default_domain,
        })
    }

    pub fn set(&self, username: &str, status_count: f64) -> Result<()> {
        let lower = username.to_lowercase();
        let mut parts = lower.splitn(2, '@');

        let (user, domain): (&str, &str) = match (parts.next(), parts.next()) {
            (Some(user), None) => (user, &self.default_domain),
            (Some(user), Some(domain)) => (user, domain),
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
            self.gauges
                .get_metric_with_label_values(&[domain, user])
                .chain_err(|| format!("failed to get metric for {}", username))?
                .set(status_count),
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

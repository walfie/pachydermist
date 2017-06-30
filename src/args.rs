use error::*;
use olifants::timeline::Endpoint;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
struct ArgsOpt {
    #[structopt(long = "instance",
                help = "Instance URL (if protocol is absent, `https` is assumed)")]
    pub instance_url: String,

    #[structopt(long = "token",
                help = "Access token. If unspecified, uses the \
                        `MASTODON_ACCESS_TOKEN` environment variable.")]
    pub access_token: Option<String>,

    #[structopt(short = "b", long = "bind", default_value = "127.0.0.1",
                help = "Server bind address for Prometheus metrics")]
    pub bind: String,

    #[structopt(short = "p", long = "port", help = "Server bind port for Prometheus metrics")]
    pub port: u16,

    #[structopt(long = "timeline", help = "Timeline type", default_value = "local",
                possible_value = "local", possible_value = "federated", possible_value = "user")]
    pub timeline: String,

    #[structopt(long = "namespace", help = "Prometheus metrics namespace (prefix)",
                default_value = "mastodon")]
    pub namespace: String,
}

pub struct Args {
    pub instance_url: String,
    pub access_token: String,
    pub bind_address: ::std::net::SocketAddr,
    pub endpoint: Endpoint,
    pub namespace: String,
}

impl Args {
    pub fn init() -> Result<Self> {
        let args = ArgsOpt::from_args();

        // If no protocol specified, assume `https://`
        let instance_url = if args.instance_url.starts_with("http://") ||
            args.instance_url.starts_with("https://")
        {
            args.instance_url
        } else {
            format!("https://{}", args.instance_url)
        };

        let access_token = args.access_token
            .or_else(|| ::std::env::var("MASTODON_ACCESS_TOKEN").ok())
            .ok_or_else(|| {
                let msg = "please specify an access token with `--token`, or by \
                       setting the `MASTODON_ACCESS_TOKEN` environment variable";
                Error::from_kind(ErrorKind::Msg(msg.into()))
            })?;

        let bind_address = {
            let string = format!("{}:{}", args.bind, args.port);
            string.parse().chain_err(
                || format!("invalid bind address {}", string),
            )
        }?;

        let endpoint = match args.timeline.as_ref() {
            "local" => Endpoint::Local,
            "federated" => Endpoint::Federated,
            "user" => Endpoint::User,
            _ => bail!("timeline must be one of: [local, federated, user]"),
        };

        Ok(Args {
            instance_url,
            access_token,
            bind_address,
            endpoint,
            namespace: args.namespace,
        })
    }
}

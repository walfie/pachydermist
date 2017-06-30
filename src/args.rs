use error::*;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
struct ArgsOpt {
    #[structopt(long = "instance",
                help = "Instance URL. If protocol is absent, `https` is assumed.")]
    pub instance_url: String,

    #[structopt(long = "token",
                help = "Access token. If unspecified, uses the \
                        `MASTODON_ACCESS_TOKEN` environment variable.")]
    pub access_token: Option<String>,

    #[structopt(short = "b", long = "bind", default_value = "127.0.0.1",
                help = "Server bind address for Prometheus metrics")]
    pub bind: String,

    #[structopt(short = "p", long = "port", help = "Server bind port for Prometheus metrics")]
    pub port: u32,
}

pub struct Args {
    pub instance_url: String,
    pub access_token: String,
    pub bind_address: ::std::net::SocketAddr,
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

        Ok(Args {
            instance_url,
            access_token,
            bind_address,
        })
    }
}

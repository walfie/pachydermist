#[derive(StructOpt, Debug)]
pub struct Opt {
    #[structopt(long = "instance", help = "Instance URL")]
    pub instance_url: String,

    #[structopt(long = "token", help = "Access token")]
    pub access_token: String,

    #[structopt(short = "b", long = "bind", default_value = "127.0.0.1",
                help = "Server bind address for Prometheus metrics")]
    pub bind: String,

    #[structopt(short = "p", long = "port", help = "Server bind port for Prometheus metrics")]
    pub port: u32,
}

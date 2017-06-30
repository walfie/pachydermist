#[derive(StructOpt, Debug)]
pub struct Opt {
    #[structopt(long = "instance", help = "Instance URL")]
    pub instance_url: String,

    #[structopt(long = "token", help = "Access token")]
    pub access_token: String,
}

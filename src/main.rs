mod environment;
mod greet;
mod login;
mod mode;
mod plymouth;
mod session;
mod systemd;
mod utils;

use anyhow::Result;
use pico_args::Arguments;

use crate::utils::config::Config;

pub const APP_NAME: &str = "flint";

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber)?;

    let mut args = Arguments::from_env();

    let config = Config::from_args(&mut args, &format!("/etc/{APP_NAME}.toml"))?;

    match args.subcommand()? {
        Some(ref subcommand) => mode::cli::run(subcommand, args, config).await,
        None => mode::daemon::run(config).await,
    }
}

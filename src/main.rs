mod bind;
mod core;
mod environment;
mod greet;
mod plug;
mod session;
mod utils;

use anyhow::{Result, bail};
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
        // Some(ref subcommand) => mode::cli::run(subcommand, args, config).await,
        Some(_) => bail!("cli mode is not supported for now"),
        None => core::daemon::run(config).await,
    }
}

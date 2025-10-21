mod environment;
mod login;
mod plymouth;
mod session;
mod utils;

use anyhow::{Context, Result, anyhow};
use pico_args::Arguments;

use crate::{
    login::context::LoginContext,
    session::{
        dispatch_session,
        manager::{SessionManager, SessionType},
    },
    utils::config::Config,
};

pub const APP_NAME: &str = "flint";

fn list<Session: SessionType>(config: &Config) -> Result<()> {
    let manager = SessionManager::<Session>::new_from_config(config)?;

    for (key, entry) in manager.lookup_metadata_all() {
        println!("{key}: {entry}")
    }

    Ok(())
}

fn run<Session: SessionType>(config: &Config, mut args: Arguments) -> Result<()> {
    let manager = SessionManager::<Session>::new_from_config(config)?;

    let name: String = args.free_from_str().map_err(|_| {
        anyhow!(
            "
                No session name provided.
                Use --list to list available sessions.    
                "
        )
    })?;

    let context = LoginContext::current(environment::current())?;
    let metadata = manager.lookup_metadata(&name)?;

    let session = manager
        .spawn_session(context, metadata.executable)
        .context("Failed to start session")?;

    let exit_reason = session.join()?;

    println!(
        "Session exited.
        Caused by: {exit_reason}"
    );

    Ok(())
}

fn daemon(_config: Config) -> Result<()> {
    todo!()
}

fn cli(subcommand: &str, mut args: Arguments, config: Config) -> Result<()> {
    let session_type = args
        .opt_value_from_str(["-s", "--session-type"])?
        .unwrap_or("x11".to_string());

    match subcommand {
        "run" => dispatch_session!(session_type.as_str() => run(&config, args)),
        "list" => dispatch_session!(session_type.as_str() => list(&config)),
        other => Err(anyhow!("Invalid subcommand: {other}")),
    }
}

fn main() -> Result<()> {
    let mut args = Arguments::from_env();

    let config = Config::from_args(&mut args, &format!("/etc/{APP_NAME}.toml"))?;

    match args.subcommand()? {
        Some(ref subcommand) => cli(subcommand, args, config),
        None => daemon(config),
    }
}

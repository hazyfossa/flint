mod environment;
mod pam;
mod session;
#[allow(dead_code)]
mod utils;

use anyhow::{Context, Result, anyhow};

use pico_args::Arguments;
use session::{
    context::SessionContext,
    dispatch_session,
    manager::{SessionManager, SessionType},
};

use crate::{
    session::manager::SessionClass,
    utils::{
        config::Config,
        runtime_dir::{self, RuntimeDir},
    },
};

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

    let context = SessionContext::from_env(environment::current())?;

    let mut child = manager
        .new_session(
            &name,
            context,
            SessionClass::User {
                early: false,
                light: false,
            },
        )
        .context("Failed to start session")?;

    let ret = child.wait()?;

    match ret.success() {
        true => Ok(()),
        false => Err(anyhow!(
            "Main session process exited with status: {}",
            ret.code()
                .and_then(|code| Some(code.to_string()))
                .unwrap_or("unknown".to_string())
        )),
    }
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
    let xdg_context = xdg::BaseDirectories::new();

    runtime_dir::current.init(
        RuntimeDir::create(&xdg_context, "flint").context("Failed to create runtime directory")?,
    )?;

    let mut args = Arguments::from_env();

    let config = Config::from_args(&mut args, "/etc/flint.toml")?;

    match args.subcommand()? {
        Some(ref subcommand) => cli(subcommand, args, config),
        None => daemon(config),
    }
}

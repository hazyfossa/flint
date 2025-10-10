mod environment;
mod login;
mod session;
#[allow(dead_code)]
mod utils;

use anyhow::{Context, Result, anyhow};

use pico_args::Arguments;
use session::{
    context::SessionContext, dispatch_session, manager::SessionManager as GenericSessionManager,
};

use crate::utils::{
    config::Config,
    runtime_dir::{self, RuntimeDir},
};

fn list<Session: GenericSessionManager>() -> Result<()> {
    for entry in Session::lookup_metadata_all() {
        println!("{}", entry)
    }

    Ok(())
}

fn run<SessionManager: GenericSessionManager>(mut args: Arguments, config: Config) -> Result<()> {
    let name: String = args.free_from_str().map_err(|_| {
        anyhow!(
            "
                No session name provided.
                Use --list to list available sessions.    
                "
        )
    })?;

    let metadata = SessionManager::lookup_metadata(&name)?;
    let manager = SessionManager::new_from_config(&config).context("Invalid session config")?;

    let context = SessionContext::from_env(environment::current())?;

    let ret = manager
        .new_session(metadata, context)
        .context("Failed to start session")?;

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
        "run" => dispatch_session!(session_type.as_str() => run(args, config)),
        "list" => dispatch_session!(session_type.as_str() => list()),
        other => Err(anyhow!("Invalid subcommand: {other}")),
    }
}

fn main() -> Result<()> {
    let xdg_context = xdg::BaseDirectories::new();

    runtime_dir::current.init(
        RuntimeDir::create(&xdg_context, "troglodyt")
            .context("Failed to create runtime directory")?,
    )?;

    let mut args = Arguments::from_env();

    let config = Config::from_args(&mut args, "/etc/troglodyt.toml")?;

    match args.subcommand()? {
        Some(ref subcommand) => cli(subcommand, args, config),
        None => daemon(config),
    }
}

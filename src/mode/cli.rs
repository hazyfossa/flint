use anyhow::{Context, Result, anyhow};
use pico_args::Arguments;

use crate::{
    environment,
    login::context::LoginContext,
    session::{define::SessionType, dispatch_session, manager::SessionManager},
    utils::config::Config,
};

async fn list<Session: SessionType>(config: &Config) -> Result<()> {
    let manager = SessionManager::<Session>::new_from_config(config)?;

    for entry in manager.lookup_metadata_all() {
        let id = &entry.id;
        print!("[{id}]");

        if let Some(name) = &entry.display_name
            && name != id
        {
            print!(": {name}")
        }

        if let Some(description) = &entry.description {
            print!(": {description}")
        }

        print!("\n")
    }

    Ok(())
}

async fn spawn_session<Session: SessionType>(config: &Config, mut args: Arguments) -> Result<()> {
    let manager = SessionManager::<Session>::new_from_config(config)?;

    #[rustfmt::skip]
    let name: String = args.free_from_str().map_err(|_| {
        anyhow!("No session name provided.
        Use --list to list available sessions.")
    })?;

    let context = LoginContext::current(environment::current())?;
    let metadata = manager.lookup_metadata(&name)?;

    let session = manager
        .spawn_session(context, &metadata.executable)
        .await
        .context("Failed to start session")?;

    let exit_reason = session.join().await?;

    #[rustfmt::skip]
    println!("Session exited.
    Caused by: {exit_reason}");

    Ok(())
}

pub async fn run(subcommand: &str, mut args: Arguments, config: Config) -> Result<()> {
    let session_type = args
        .opt_value_from_str(["-s", "--session-type"])?
        .unwrap_or("x11".to_string());

    match subcommand {
        "run" => dispatch_session!(session_type.as_str() => spawn_session(&config, args)),
        "list" => dispatch_session!(session_type.as_str() => list(&config)),
        other => Err(anyhow!("Invalid subcommand: {other}")),
    }
}

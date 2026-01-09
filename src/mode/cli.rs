use anyhow::{Context, Result, anyhow};
use pico_args::Arguments;

use crate::{
    environment, login::context::LoginContext, session::SessionManager, utils::config::Config,
};

async fn list(config: &Config) -> Result<()> {
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

async fn spawn_session(manager: SessionTypeData, mut args: Arguments) -> Result<()> {
    #[rustfmt::skip]
    let name: String = args.free_from_str().map_err(|_| {
        anyhow!("No session name provided.
        Use --list to list available sessions.")
    })?;

    let context = LoginContext::current(environment::current())?;
    let metadata = manager.lookup_metadata(&name)?;

    let session = manager
        .run(context, &metadata.executable)
        .await
        .context("Failed to start session")?;

    let exit_reason = session.join().await?;

    #[rustfmt::skip]
    println!("Session exited.
    Caused by: {exit_reason}");

    Ok(())
}

pub async fn run(subcommand: &str, mut args: Arguments, config: Config) -> Result<()> {
    let manager = SessionTypeData::parse().config(&config);

    if let Some(value) = args.opt_value_from_str(["-s", "--session-type"])? {
        manager = manager.tag(value);
    };

    match subcommand {
        "run" => spawn_session(session_type_tag, &config, args).await,
        "list" => list(session_type_tag, &config).await,
        other => Err(anyhow!("Invalid subcommand: {other}")),
    }
}

mod environment;
mod login;
mod template;
#[allow(dead_code)]
mod tty;
mod utils;
mod wayland;
mod x;

use anyhow::{Context, Result, anyhow};

use pico_args::Arguments;
use template::{Session, SessionManager};

use crate::utils::{
    config::Config,
    runtime_dir::{self, RuntimeDir},
};

crate::define_env!("XDG_SEAT", Seat(String));

impl Default for Seat {
    fn default() -> Self {
        // man sd-login says that seat0 always exists
        Self("seat0".into())
    }
}

crate::sessions!([x::Session, wayland::Session]);

fn list<Session: template::Session>() -> Result<()> {
    for entry in Session::lookup_all() {
        println!("{}", entry)
    }

    Ok(())
}

fn run<Session: template::Session>(mut args: Arguments, config: Config) -> Result<()> {
    let name: String = args.free_from_str().map_err(|_| {
        anyhow!(
            "
                No session name provided.
                Use --list to list available sessions.    
                "
        )
    })?;

    let metadata = Session::lookup(&name)?;
    let manager = Session::Manager::new_from_config(&config).context("Invalid session config")?;

    let ret = manager.start(metadata).context("Failed to start session")?;

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
        .unwrap_or(x::Session::XDG_TYPE.to_string());

    match subcommand {
        "run" => crate::dispatch_session!(session_type.as_str() => run(args, config)),
        "list" => crate::dispatch_session!(session_type.as_str() => list()),
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

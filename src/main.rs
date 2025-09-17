#![allow(dead_code)]

mod console;
mod environment;
mod login;
mod template;
mod utils;
mod wayland;
mod x;

use anyhow::{Result, anyhow};
use pico_args::Arguments;

use template::{Session, SessionManager, SessionMetadata};
use utils::runtime_dir::RuntimeDir;

crate::define_env!("XDG_SEAT", Seat(String));

impl Default for Seat {
    fn default() -> Self {
        // man sd-login says that seat0 always exists
        Self("seat0".into())
    }
}

fn run<T: Session>(mut args: Arguments) -> Result<()> {
    if args.contains("--list") {
        for entry in SessionMetadata::<T>::lookup_all() {
            println!("{}", entry)
        }
        Ok(())
    } else {
        let name: String = args.free_from_str().map_err(|_| {
            anyhow!(
                "
            No session name provided.
            Use --list to list available sessions.    
    "
            )
        })?;

        let metadata = SessionMetadata::<T>::lookup(&name)?;
        let manager = T::Manager::new_from_config()?;
        manager.start(metadata)
    }
}

fn main() -> Result<()> {
    let xdg_context = xdg::BaseDirectories::new();
    let runtime_dir = RuntimeDir::new(&xdg_context, "troglodyt")?;

    let mut args = Arguments::from_env();

    let subcommand = args.subcommand()?;
    let session_type_arg = match subcommand {
        Some(ref arg) => arg.as_str(),
        None => "x11",
    };

    match session_type_arg {
        x::Session::XDG_TYPE => run::<x::Session>(args),
        other => Err(anyhow!("{other} is not a valid session type.")),
    }
}

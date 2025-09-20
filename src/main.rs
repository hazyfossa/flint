mod environment;
mod login;
mod template;
#[allow(dead_code)]
mod tty;
mod users;
mod utils;
mod wayland;
mod x;

use anyhow::{Result, anyhow};
use pico_args::Arguments;

use template::{Session, SessionManager};

crate::define_env!("XDG_SEAT", Seat(String));

impl Default for Seat {
    fn default() -> Self {
        // man sd-login says that seat0 always exists
        Self("seat0".into())
    }
}

fn run<Session: template::Session>(mut args: Arguments) -> Result<()> {
    if args.contains("--list") {
        for entry in Session::lookup_all() {
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

        let metadata = Session::lookup(&name)?;
        let manager = Session::Manager::new_from_config()?;
        let ret = manager.start(metadata)?;

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
}

macro_rules! dispatch_session {
    ($xdg_type:expr, $args:expr, [$($session:ty),+]) => {
        match $xdg_type {
            $(
                <$session>::XDG_TYPE => run::<$session>($args),
            )+
            other => Err(anyhow!("{other} is not a valid session type.")),
        }
    };
}

fn main() -> Result<()> {
    let mut args = Arguments::from_env();

    let subcommand = args.subcommand()?;
    let session_type_arg = match subcommand {
        Some(ref arg) => arg.as_str(),
        None => "x11",
    };

    dispatch_session!(session_type_arg, args, [x::Session, wayland::Session])
}

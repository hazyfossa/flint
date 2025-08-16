#![allow(dead_code)]

mod console;
mod environment;
mod login;
mod session;
mod utils;
mod x;

use anyhow::{Context, Result};
use environment::{EnvContext, EnvValue};
use session::{DesktopRunner, SessionType};
use utils::runtime_dir::RuntimeDir;

struct Seat(String);

impl EnvValue for Seat {
    const KEY: &str = "XDG_SEAT";
    crate::env_impl!();
}

impl Default for Seat {
    fn default() -> Self {
        // man sd-login says that seat0 always exists
        Self("seat0".into())
    }
}

fn main() -> Result<()> {
    let mut env = EnvContext::current();

    let xdg_context = xdg::BaseDirectories::new();
    let runtime_dir = RuntimeDir::new(&xdg_context, "troglodyt")?;

    let session_name = "i3";
    let session =
        x::Session::lookup(session_name).context(format!("Cannot find session {session_name}"))?;

    let x_server = x::setup(&mut env, &runtime_dir).context("Failed to start Xorg")?;

    let runner = DesktopRunner::new(session, env);
    runner.start_main()?.ok()
}

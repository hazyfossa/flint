#![allow(dead_code)]

mod console;
mod environment;
mod login;
mod template;
mod utils;
mod wayland;
mod x;

use std::path::PathBuf;

use anyhow::Result;
use utils::runtime_dir::RuntimeDir;

use crate::template::SessionManager;

crate::define_env!("XDG_SEAT", Seat(String));

impl Default for Seat {
    fn default() -> Self {
        // man sd-login says that seat0 always exists
        Self("seat0".into())
    }
}

fn main() -> Result<()> {
    let xdg_context = xdg::BaseDirectories::new();
    let runtime_dir = RuntimeDir::new(&xdg_context, "troglodyt")?;

    let session_name = "i3";

    let session_manager =
        x::SessionManager::with_config(PathBuf::from("/usr/lib/Xorg"), runtime_dir);

    session_manager.start(session_name)
}

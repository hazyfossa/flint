#![allow(dead_code)]
use std::path::Path;

use super::define::prelude::*;
use crate::environment::prelude::*;

define_env!("WAYLAND_DISPLAY", pub Display(String));
env_parser_auto!(Display);

pub struct Session;

impl metadata::FreedesktopMetadata for Session {
    const LOOKUP_PATH: &str = "/usr/share/wayland-sessions/";
}

#[derive(Deserialize)]
#[serde(default)]
pub struct Config {}

impl Default for Config {
    fn default() -> Self {
        Self {}
    }
}

impl define::SessionType for Session {
    const XDG_TYPE: &str = "wayland";

    type ManagerConfig = Config;

    async fn setup_session(
        _config: &Config,
        _context: &mut SessionContext,
        _executable: &Path,
    ) -> anyhow::Result<()> {
        todo!()
    }
}

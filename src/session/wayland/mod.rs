use super::manager::prelude::*;
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

impl manager::SessionType for Session {
    const XDG_TYPE: &str = "wayland";

    type ManagerConfig = Config;
    type EnvDiff = Display;

    fn setup_session(_config: &Config, _context: SessionContext) -> anyhow::Result<Self::EnvDiff> {
        todo!()
    }
}

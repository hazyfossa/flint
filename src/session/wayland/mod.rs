use super::manager::prelude::*;
use crate::define_env;

define_env!("WAYLAND_DISPLAY", pub Display(String));

impl FreedesktopMetadata for SessionManager {
    const LOOKUP_PATH: &str = "/usr/share/wayland-sessions/";
}

#[derive(Deserialize)]
pub struct SessionManager;

impl Default for SessionManager {
    fn default() -> Self {
        Self {}
    }
}

impl manager::SessionManager for SessionManager {
    const XDG_TYPE: &str = "wayland";
    type Env = Display;

    fn setup_session(&self, _context: SessionContext) -> anyhow::Result<Self::Env> {
        todo!()
    }
}

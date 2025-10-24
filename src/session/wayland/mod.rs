#![allow(dead_code)]
use std::path::Path;

use anyhow::Result;
use tokio::process::Command;

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
        context: &mut SessionContext,
        executable: &Path,
    ) -> Result<()> {
        context.update_env((
            "MOZ_ENABLE_WAYLAND=1",
            "QT_QPA_PLATFORM=wayland",
            "SDL_VIDEODRIVER=wayland",
            "_JAVA_AWT_WM_NONREPARENTING=1",
        ));

        // TODO: anything else?
        context.spawn(Command::new(executable))
    }
}

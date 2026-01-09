#![allow(dead_code)]
use std::path::Path;

use anyhow::Result;
use facet::Facet;
use tokio::process::Command;

use crate::{
    environment::prelude::*,
    session::{SessionType, manager::SessionContext, metadata::FreedesktopMetadata},
};

define_env!("WAYLAND_DISPLAY", pub Display(String));
env_parser_auto!(Display);

#[derive(Facet, Default)]
pub struct SessionConfig;

impl FreedesktopMetadata for SessionConfig {
    const LOOKUP_PATH: &str = "/usr/share/wayland-sessions/";
}

#[async_trait::async_trait]
impl SessionType for SessionConfig {
    fn tag(&self) -> &'static str {
        "wayland"
    }

    async fn setup_session(&self, context: &mut SessionContext, executable: &Path) -> Result<()> {
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

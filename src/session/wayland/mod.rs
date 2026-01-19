#![allow(dead_code)]
use std::path::Path;

use anyhow::Result;
use facet::Facet;
use tokio::process::Command;

use crate::{environment::prelude::*, session::define::prelude::*};

define_env!("WAYLAND_DISPLAY", pub Display(String));
env_parser_auto!(Display);

#[derive(Facet, Default)]
pub struct SessionManager;

impl FreedesktopMetadata for SessionManager {
    const LOOKUP_PATH: &str = "/usr/share/wayland-sessions/";
}

impl SessionType for SessionManager {
    const TAG: &SessionTypeTag<str> = "wayland";

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

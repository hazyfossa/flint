#![allow(dead_code)]
use std::path::Path;

use anyhow::Result;
use facet::Facet;
use tokio::process::Command;

use crate::{environment::define_env, session::define::prelude::*};

define_env!(pub Display(String) = parse "WAYLAND_DISPLAY");

#[derive(Facet, Default)]
pub struct SessionManager;

impl FreedesktopMetadata for SessionManager {
    const LOOKUP_PATH: &str = "/usr/share/wayland-sessions/";
}

impl SessionType for SessionManager {
    const TAG: &SessionTypeTag<str> = "wayland";

    async fn setup_session(&self, context: &mut SessionContext, executable: &Path) -> Result<()> {
        context.env.set((
            "MOZ_ENABLE_WAYLAND=1",
            "QT_QPA_PLATFORM=wayland",
            "SDL_VIDEODRIVER=wayland",
            "_JAVA_AWT_WM_NONREPARENTING=1",
        ));

        // TODO: anything else?
        context.spawn(Command::new(executable))
    }
}

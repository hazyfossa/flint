use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use facet::Facet;
use tokio::process::Command;

use crate::{
    login::{VtRenderMode, tty::Terminal, users::env::Shell},
    session::{
        define::prelude::*,
        metadata::{SessionMetadata, SessionMetadataLookup, SessionMetadataMap},
    },
};

#[derive(Default, Facet)]
#[facet(default)]
pub struct SessionManager;

impl SessionType for SessionManager {
    const TAG: &SessionTypeTag<str> = "tty";

    const VT_RENDER_MODE: VtRenderMode = VtRenderMode::Text;

    async fn setup_session(&self, context: &mut SessionContext, executable: &Path) -> Result<()> {
        // TODO: does it make sense to try and allocate one here?
        let vt = context
            .vt
            .take()
            .ok_or(anyhow!("Cannot start a TTY session without a VT"))?;

        let terminal = Terminal::new(vt).context("Cannot open VT by number")?;

        let executable = if executable == PathBuf::from("<shell_env>") {
            &*context
                .env
                .peek::<Shell>()
                .context("Cannot find user shell")?
        } else {
            executable
        };

        let mut cmd = Command::new(executable);

        // TODO: is this necessary?
        // in case of PAM the context-child will inherit the tty stdin
        // but what about cli?
        unsafe {
            cmd.pre_exec(move || Ok(terminal.raw.bind_stdio()?));
        }

        context.spawn(cmd)
    }
}

fn special_meta_shell() -> SessionMetadata<SessionManager> {
    SessionMetadata::builder()
        .id("shell".into())
        .description("Default shell as set for the target user".into())
        .executable("<shell_env>".into())
        .build()
}

impl SessionMetadataLookup for SessionManager {
    type T = SessionManager;

    fn lookup_metadata(&self, name: &str) -> Result<SessionMetadata<Self::T>> {
        match name {
            "shell" => Ok(special_meta_shell()),

            _ => Err(anyhow!(
                r#"Arbitrary executables are not supported as a tty session.
            Create a new entry in the config."#
            )),
        }
    }

    fn lookup_metadata_all(&self) -> SessionMetadataMap<Self::T> {
        // NOTE: at least debian provides a list of valid shells

        // This is a hack
        let mut map = SessionMetadataMap::new();
        map.insert(special_meta_shell());
        map
    }
}

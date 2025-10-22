use std::{path::PathBuf, process::Command};

use anyhow::{Context, Result, anyhow};

use super::define::prelude::*;
use crate::login::{VtRenderMode, users::env::Shell};

pub struct Session;

#[derive(Default, Deserialize)]
#[serde(default)]
pub struct Config {}

impl define::SessionType for Session {
    const XDG_TYPE: &str = "tty";

    type ManagerConfig = Config;

    const VT_RENDER_MODE: VtRenderMode = VtRenderMode::Text;

    fn setup_session(
        _config: &Config,
        context: &mut SessionContext,
        executable: PathBuf,
    ) -> Result<()> {
        let executable = if executable == PathBuf::from("shell_env") {
            &*context
                .env
                .peek::<Shell>()
                .context("Cannot find user shell")?
        } else {
            &executable
        };

        // TODO: should we pass Seat here too?
        context.update_env(context.terminal.number);

        context.spawn(Command::new(executable))
    }
}

fn special_meta_shell() -> SessionMetadata {
    SessionMetadata {
        name: "shell".to_string(),
        description: Some("Default shell as set for the target user".to_string()),
        executable: PathBuf::from("<shell_env>"),
    }
}

impl metadata::SessionMetadataLookup for Session {
    fn lookup_metadata(name: &str) -> Result<SessionMetadata> {
        match name {
            "shell" => Ok(special_meta_shell()),

            _ => Err(anyhow!(
                r#"Arbitrary executables are not supported as a tty session.
            Create a new entry in the config."#
            )),
        }
    }

    fn lookup_metadata_all() -> SessionMap {
        // NOTE: at least debian provides a list of valid shells

        // This is a hack
        SessionMap::new().update("shell".to_string(), special_meta_shell())
    }
}

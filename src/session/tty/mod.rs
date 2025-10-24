use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use tokio::process::Command;

use super::define::prelude::*;
use crate::{
    login::{VtRenderMode, users::env::Shell},
    session::metadata::{SessionDefinition, SessionMetadata},
};

pub struct Session;

#[derive(Default, Deserialize)]
#[serde(default)]
pub struct Config {}

impl define::SessionType for Session {
    const XDG_TYPE: &str = "tty";

    type ManagerConfig = Config;

    const VT_RENDER_MODE: VtRenderMode = VtRenderMode::Text;

    async fn setup_session(
        _config: &Config,
        context: &mut SessionContext,
        executable: &Path,
    ) -> Result<()> {
        let terminal = context
            .terminal
            .take()
            .ok_or(anyhow!("Failed to aqquire terminal from context"))?;

        let executable = if executable == PathBuf::from("<shell_env>") {
            &*context
                .env
                .peek::<Shell>()
                .context("Cannot find user shell")?
        } else {
            executable
        };

        // TODO: should we pass Seat here too?
        context.update_env(terminal.number);

        let mut cmd = Command::new(executable);

        unsafe {
            cmd.pre_exec(move || Ok(terminal.raw.bind_stdio()?));
        }

        context.spawn(cmd)
    }
}

fn special_meta_shell() -> SessionDefinition {
    SessionDefinition::from_meta(
        "shell".to_string(),
        SessionMetadata {
            display_name: None,
            description: Some("Default shell as set for the target user".to_string()),
            executable: PathBuf::from("<shell_env>"),
        },
    )
}

impl metadata::SessionMetadataLookup for Session {
    fn lookup_metadata(name: &str) -> Result<SessionDefinition> {
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
        SessionMap::new().update(special_meta_shell())
    }
}

use std::path::PathBuf;

use anyhow::{Result, anyhow};

use super::manager::prelude::*;
use crate::login::{VtRenderMode, context::VtNumber};

pub struct Session;

#[derive(Default, Deserialize)]
#[serde(default)]
pub struct Config {}

impl manager::SessionType for Session {
    const XDG_TYPE: &str = "tty";

    type ManagerConfig = Config;
    type EnvDiff = VtNumber;

    const VT_RENDER_MODE: VtRenderMode = VtRenderMode::Text;

    fn setup_session(_config: &Config, context: &mut SessionContext) -> Result<Self::EnvDiff> {
        Ok(context.vt.clone())
    }
}

impl metadata::SessionMetadataLookup for Session {
    fn lookup_metadata(_name: &str) -> Result<SessionMetadata> {
        Err(anyhow!(
            r#"Arbitrary executables are not supported as a tty session.
            Create a new entry in the config."#
        ))
    }

    fn lookup_metadata_all() -> SessionMap {
        // NOTE: at least debian provides a list of valid shells

        // This is a hack
        SessionMap::new().update(
            "shell".to_string(),
            SessionMetadata {
                name: "shell".to_string(),
                description: Some("Default shell as set for the target user".to_string()),
                executable: PathBuf::from("<set_externally>"),
            },
        )
    }
}

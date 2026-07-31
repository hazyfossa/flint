use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};

mod xdg;

// An opaque session metadata identifier
// also known as a handle
// TODO: consider an atomic counter instead
pub type ID = uuid::Uuid;

// TODO: ponder if this can be per-user (probably a bad idea)
// why specifically a bad idea:
// 1. safety semantics are tricky (but not impossible)
// 2. due to systemd-homed, we can only descend into /home after `pam`
// after thinking for a bit, though, all of this is solvable
#[derive(Serialize, Deserialize, Clone)]
enum Source {
    XDG,
    Distribution,
    GlobalConfig,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Summary {
    name: String,
    description: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    #[serde(flatten)]
    summary: Summary,

    // TODO: this should refer to the SessionManagerObject
    // config as we inject it
    #[serde(rename = "config")]
    ref_session_config: String,

    executable: String,
}

pub type IntrinsicTag = &'static str;

enum Definition {
    Intrinsic {
        tag: IntrinsicTag,
    },

    External {
        source: Source,
        source_path: PathBuf,
        executable: PathBuf,
    },
}

impl Definition {
    fn source(&self) -> Option<&Source> {
        match self {
            Self::External { source, .. } => Some(source),
            Self::Intrinsic { .. } => None,
        }
    }
}

pub struct Metadata {
    summary: Summary,
    definition: Definition,
}

impl Metadata {
    fn for_greeter(&self) -> ForGreeter<'_> {
        ForGreeter {
            summary: &self.summary,
            source: self.definition.source(),
        }
    }
}

pub struct DefinedSessions {
    store: HashMap<ID, Metadata>,
}

#[derive(Serialize)]
pub struct ForGreeter<'a> {
    #[serde(flatten)]
    summary: &'a Summary,
    source: Option<&'a Source>,
}

impl DefinedSessions {
    fn for_greeter(&self) -> HashMap<&ID, ForGreeter<'_>> {
        self.store
            .iter()
            .map(|(k, v)| (k, v.for_greeter()))
            .collect()
    }
}

pub async fn load(config: super::Config) -> DefinedSessions {
    todo!()
}

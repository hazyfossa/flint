use anyhow::{Context, Result, anyhow, bail};
use freedesktop_file_parser::{self as parser, EntryType};
use fs_err::{DirEntry, File, read_dir};
use im::HashMap;
use serde::Deserialize;
use shrinkwraprs::Shrinkwrap;

use std::{
    io::{self, ErrorKind, Read},
    path::PathBuf,
    vec,
};

use crate::environment::{EnvContainer, prelude::*};

#[derive(Deserialize, Clone)]
pub struct SessionMetadata {
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub executable: PathBuf,
}

#[derive(Shrinkwrap)]
pub struct SessionDefinition {
    pub id: String,
    #[shrinkwrap(main_field)]
    metadata: SessionMetadata,
}

impl SessionDefinition {
    pub fn from_meta(id: String, metadata: SessionMetadata) -> Self {
        Self { id, metadata }
    }
}

pub trait SessionMetadataLookup {
    fn lookup_metadata(name: &str) -> Result<SessionDefinition>;

    /// This function will return metadata for all available sessions
    /// It is currently not guaranteed that a session can be started for each entry
    /// I.e. the metadata can specify an executable that is unavailable.
    ///
    /// Entries with invalid metadata are silently discarded.
    fn lookup_metadata_all() -> SessionMap;
}

pub trait FreedesktopMetadata {
    const LOOKUP_PATH: &str;
}

fn parse_freedesktop_file(file: &mut File) -> Result<SessionDefinition> {
    let id = file
        .path()
        .file_name()
        .unwrap()
        .to_str()
        .ok_or(anyhow!("Invalid filename for a session file"))?
        .to_string();

    let mut buf = String::new();
    file.read_to_string(&mut buf)?;

    let parsed = parser::parse(&buf)?.entry;

    let app = match parsed.entry_type {
        EntryType::Application(app) => app,
        x => bail!("Not a valid entry type for a session: {x}",),
    };

    // TODO: does it make sense to check for try_exec here?
    let executable = app
        .exec
        .ok_or(anyhow!("Session does not define an executable"))?
        .into();

    let metadata = SessionMetadata {
        display_name: Some(parsed.name.default),
        description: parsed.comment.map(|x| x.default),
        executable,
    };

    Ok(SessionDefinition { id, metadata })
}

impl<T: FreedesktopMetadata> SessionMetadataLookup for T {
    fn lookup_metadata(id: &str) -> Result<SessionDefinition> {
        let path = PathBuf::from(Self::LOOKUP_PATH).join(format!("{id}.desktop"));

        let mut file = File::open(path).map_err(|e| match e.kind() {
            ErrorKind::NotFound => anyhow!("Such a session is not defined"),
            _ => e.into(),
        })?;

        parse_freedesktop_file(&mut file).context("Session definition is incorrect")
    }

    fn lookup_metadata_all() -> SessionMap {
        // TODO: consider reporting errors on parsing failure

        let dir = match read_dir(Self::LOOKUP_PATH) {
            Ok(dir) => dir,
            Err(_) => return SessionMap::new(),
        };

        fn files(entry: io::Result<DirEntry>) -> Option<File> {
            let entry = entry.ok()?;
            if !entry.metadata().ok()?.is_file() {
                return None;
            }

            File::open(entry.path()).ok()
        }

        dir.filter_map(files)
            .filter_map(|mut file| parse_freedesktop_file(&mut file).ok())
            .collect()
    }
}

define_env!("XDG_SESSION_DESKTOP", SessionNameEnv(String));
env_parser_auto!(SessionNameEnv);

// TODO: investigate how this can contain more than one name
define_env!("XDG_CURRENT_DESKTOP", SessionCompositionEnv(Vec<String>));

impl SessionCompositionEnv {
    fn simple(name: String) -> Self {
        Self(vec![name])
    }
}

impl EnvParser for SessionCompositionEnv {
    fn serialize(&self) -> std::ffi::OsString {
        self.0.join(";").into()
    }

    fn deserialize(value: std::ffi::OsString) -> Result<Self> {
        Ok(Self(
            value
                .try_to_string()?
                .split(';')
                .map(String::from)
                .collect(),
        ))
    }
}

impl EnvContainer for SessionDefinition {
    fn apply_as_container(self, env: Env) -> Env {
        // TODO: is this correct per spec?
        let name = self.display_name.as_ref().unwrap_or(&self.id);

        env.set(SessionNameEnv(name.to_string()))
            .set(SessionCompositionEnv::simple(name.to_string()))
    }
}

#[derive(Deserialize, Clone)]
pub struct SessionMap(HashMap<String, SessionMetadata>);

impl SessionMap {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn get(&self, id: &str) -> Option<SessionDefinition> {
        self.0
            .get(id)
            .map(|meta| SessionDefinition::from_meta(id.to_string(), meta.clone()))
    }

    pub fn union(self, other: SessionMap) -> SessionMap {
        Self(self.0.union(other.0))
    }

    pub fn update(&mut self, value: SessionDefinition) -> Self {
        Self(self.0.update(value.id, value.metadata))
    }
}

impl FromIterator<SessionDefinition> for SessionMap {
    fn from_iter<T: IntoIterator<Item = SessionDefinition>>(iter: T) -> Self {
        Self(
            iter.into_iter()
                .map(|meta| (meta.id.clone(), meta.clone() as _))
                .collect(),
        )
    }
}

impl IntoIterator for SessionMap {
    type IntoIter = std::vec::IntoIter<SessionDefinition>;
    type Item = SessionDefinition;

    fn into_iter(self) -> Self::IntoIter {
        self.0
            .iter()
            .map(|(k, v)| SessionDefinition::from_meta(k.to_string(), v.clone()))
            .collect::<Vec<_>>()
            .into_iter()
    }
}

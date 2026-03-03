use anyhow::{Context, Result, anyhow, bail};
use bon::Builder;
use facet::Facet;
use freedesktop_file_parser::{self as parser, EntryType};
// TODO: async dir iter
use fs_err::{DirEntry, File, read_dir};

use std::{
    collections::HashMap,
    io::{self, ErrorKind, Read},
    path::PathBuf,
    vec,
};

use crate::{
    frame::environment::{EnvironmentParse, define_env},
    session::SessionTypePlug,
};

#[derive(Facet, Clone, Builder)]
pub struct SessionMetadata<T> {
    pub id: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub executable: PathBuf,
    pub config: Option<T>,
}

pub trait SessionMetadataLookup {
    type T: SessionTypePlug;

    fn lookup_metadata(&self, id: &str) -> Result<SessionMetadata<Self::T>>;

    /// This function will return metadata for all available sessions
    /// It is currently not guaranteed that a session can be started for each entry
    /// I.e. the metadata can specify an executable that is unavailable.
    ///
    /// Entries with invalid metadata are silently discarded.
    fn lookup_metadata_all(&self) -> SessionMetadataMap<Self::T>;
}

pub trait FreedesktopMetadata {
    const LOOKUP_PATH: &str;
}

fn parse_freedesktop_file<T>(file: &mut File) -> Result<SessionMetadata<T>> {
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

    let metadata = SessionMetadata::builder()
        .id(id)
        .display_name(parsed.name.default)
        .maybe_description(parsed.comment.map(|x| x.default))
        .executable(executable)
        .build();

    Ok(metadata)
}

impl<T: FreedesktopMetadata + SessionTypePlug> SessionMetadataLookup for T {
    type T = Self;

    fn lookup_metadata(&self, id: &str) -> Result<SessionMetadata<Self::T>> {
        let path = PathBuf::from(Self::LOOKUP_PATH).join(format!("{id}.desktop"));

        let mut file = File::open(path).map_err(|e| match e.kind() {
            ErrorKind::NotFound => anyhow!("Such a session is not defined"),
            _ => e.into(),
        })?;

        parse_freedesktop_file(&mut file).context("Session definition is incorrect")
    }

    fn lookup_metadata_all(&self) -> SessionMetadataMap<Self::T> {
        // TODO: consider reporting errors on parsing failure

        let dir = match read_dir(Self::LOOKUP_PATH) {
            Ok(dir) => dir,
            Err(_) => return SessionMetadataMap::new(),
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

define_env!(pub SessionNameEnv(String) = parse "XDG_SESSION_DESKTOP");

// TODO: investigate how this can contain more than one name
define_env!(pub SessionCompositionEnv(Vec<String>) = "XDG_CURRENT_DESKTOP");

impl SessionCompositionEnv {
    fn simple(name: String) -> Self {
        Self(vec![name])
    }
}

impl EnvironmentParse<String> for SessionCompositionEnv {
    fn env_serialize(self) -> String {
        self.0.join(";")
    }

    fn env_deserialize(value: String) -> Result<Self> {
        Ok(Self(value.split(';').map(String::from).collect()))
    }
}

#[derive(Facet, Clone, Default)]
pub struct SessionMetadataMap<T> {
    entries: HashMap<String, SessionMetadata<T>>,
}

impl<T: SessionTypePlug> SessionMetadataMap<T> {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn get(&self, id: &str) -> Option<&SessionMetadata<T>> {
        self.entries.get(id)
    }

    pub fn extend(&mut self, other: SessionMetadataMap<T>) {
        self.entries.extend(other.entries)
    }

    pub fn insert(&mut self, value: SessionMetadata<T>) {
        self.entries.insert(value.id.clone(), value);
    }
}

impl<T> FromIterator<SessionMetadata<T>> for SessionMetadataMap<T> {
    fn from_iter<I: IntoIterator<Item = SessionMetadata<T>>>(iter: I) -> Self {
        Self {
            entries: iter
                .into_iter()
                .map(|meta| (meta.id.clone(), meta))
                .collect(),
        }
    }
}

use anyhow::{Context, Result, anyhow, bail};
use bon::Builder;
use facet::Facet;
use facet_value::Value;
use freedesktop_file_parser::{self as parser, EntryType};
use fs_err::{DirEntry, File, read_dir};
use shrinkwraprs::Shrinkwrap;

use std::{
    collections::HashMap,
    io::{self, ErrorKind, Read},
    marker::PhantomData,
    path::PathBuf,
    vec,
};

use crate::{
    environment::{EnvContainerPartial, prelude::*},
    session::{SessionType, SessionTypeTag},
};

#[derive(Facet, Clone, Builder)]
pub struct SessionMetadata {
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub executable: PathBuf,

    #[facet(flatten)]
    #[builder(default)]
    pub other: HashMap<String, Value>,
}

// TODO
#[derive(Shrinkwrap)]
pub struct SessionDefinition<T: SessionType> {
    _type: PhantomData<T>,
    pub id: String,
    #[shrinkwrap(main_field)]
    pub metadata: SessionMetadata,
}

impl<T: SessionType> SessionDefinition<T> {
    fn from_meta(id: String, metadata: SessionMetadata) -> Self {
        Self {
            _type: PhantomData,
            id,
            metadata,
        }
    }
}

pub trait SessionMetadataLookup {
    type T: SessionType;

    fn lookup_metadata(&self, id: &str) -> Result<SessionDefinition<Self::T>>;

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

    let metadata = SessionMetadata::builder()
        .display_name(parsed.name.default)
        .maybe_description(parsed.comment.map(|x| x.default))
        .executable(executable)
        .build();

    Ok(SessionDefinition { id, metadata })
}

impl<T: FreedesktopMetadata> SessionMetadataLookup for T {
    fn lookup_metadata(&self, id: &str) -> Result<SessionDefinition> {
        let path = PathBuf::from(Self::LOOKUP_PATH).join(format!("{id}.desktop"));

        let mut file = File::open(path).map_err(|e| match e.kind() {
            ErrorKind::NotFound => anyhow!("Such a session is not defined"),
            _ => e.into(),
        })?;

        parse_freedesktop_file(&mut file).context("Session definition is incorrect")
    }

    fn lookup_metadata_all(&self) -> SessionMetadataMap {
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

impl EnvContainerPartial for SessionDefinition {
    fn apply_as_container(&self, env: Env) -> Env {
        // TODO: is this correct per spec?
        let name = self.display_name.as_ref().unwrap_or(&self.id);

        env.set(SessionNameEnv(name.to_string()))
            .set(SessionCompositionEnv::simple(name.to_string()))
    }
}

#[derive(Facet, Clone, Default)]
pub struct SessionMetadataMap<T: SessionType> {
    _type: PhantomData<T>,
    entries: HashMap<String, SessionMetadata>,
}

impl<T: SessionType> SessionMetadataMap<T> {
    pub fn new(tag: SessionTypeTag) -> Self {
        Self {
            tag,
            entries: HashMap::new(),
        }
    }

    pub fn get(&self, id: &str) -> Option<SessionDefinition> {
        self.entries.get(id).map(|meta| SessionDefinition {
            tag: self.tag.clone(), // TODO: no clone
            id: id.to_string(),
            metadata: meta.clone(),
        })
    }

    pub fn extend(&mut self, other: SessionMetadataMap) {
        // TODO: this would be compile-time if we haven't dropped session typestate
        // in favor of runtime resolution
        if self.tag != other.tag {
            panic!(
                "Attempted merging metadata maps of different session types: {} + {}",
                self.tag, other.tag
            )
        }

        self.entries.extend(other.entries)
    }

    pub fn insert(&mut self, value: SessionDefinition) {
        self.entries.insert(value.id, value.metadata);
    }
}

// impl FromIterator<SessionDefinition> for SessionMetadataMap {
//     fn from_iter<T: IntoIterator<Item = SessionDefinition>>(iter: T) -> Self {
//         Self(
//             iter.into_iter()
//                 .map(|meta| (meta.id.clone(), meta.clone() as _))
//                 .collect(),
//         )
//     }
// }

// impl IntoIterator for SessionMetadataMap {
//     type IntoIter = std::vec::IntoIter<SessionDefinition>;
//     type Item = SessionDefinition;

//     fn into_iter(self) -> Self::IntoIter {
//         self.0
//             .iter()
//             .map(|(k, v)| SessionDefinition {
//                 id: k.to_string(),
//                 metadata: v.clone(),
//             })
//             .collect::<Vec<_>>()
//             .into_iter()
//     }
// }

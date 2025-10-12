use freedesktop_file_parser::{self as parser, EntryType};
use fs_err::{DirEntry, File, read_dir};

use anyhow::{Context, Result, anyhow, bail};
use im::HashMap;
use serde::Deserialize;

use std::{
    fmt::Display,
    io::{self, ErrorKind, Read},
    path::PathBuf,
};

pub type SessionTypeID = String;
pub type SessionMap = HashMap<SessionTypeID, SessionMetadata>;

#[derive(Clone, Deserialize)]
pub struct SessionMetadata {
    /// This is the "display" name
    /// It may be different from the SessionTypeID
    pub name: String,
    pub description: Option<String>,
    pub executable: PathBuf,
}

pub trait SessionMetadataLookup {
    fn lookup_metadata(name: &str) -> Result<SessionMetadata>;

    /// This function will return metadata for all available sessions
    /// It is currently not guaranteed that a session can be started for each entry
    /// I.e. the metadata can specify an executable that is unavailable.
    ///
    /// Entries with invalid metadata are silently discarded.
    fn lookup_metadata_all() -> HashMap<SessionTypeID, SessionMetadata>;
}

pub trait FreedesktopMetadata {
    const LOOKUP_PATH: &str;
}

fn parse_freedesktop_file(file: &mut File) -> Result<SessionMetadata> {
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

    Ok(SessionMetadata {
        name: parsed.name.default,
        description: parsed.comment.map(|x| x.default),
        executable,
    })
}

impl<T: FreedesktopMetadata> SessionMetadataLookup for T {
    fn lookup_metadata(name: &str) -> Result<SessionMetadata> {
        let path = PathBuf::from(Self::LOOKUP_PATH).join(format!("{name}.desktop"));

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

        fn with_filename(file: File) -> Option<(String, File)> {
            Some((file.path().file_name()?.to_str()?.to_string(), file))
        }

        dir.filter_map(files)
            .filter_map(with_filename)
            .filter_map(|(filename, mut file)| {
                Some((filename, parse_freedesktop_file(&mut file).ok()?))
            })
            .collect()
    }
}

impl Display for SessionMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)?;
        if let Some(comment) = &self.description {
            write!(f, ": {}", comment)?;
        };
        writeln!(f, "")
    }
}

use anyhow::{Context, Result, anyhow, bail};
use freedesktop_file_parser::{self as parser, EntryType};

use std::{
    fs::{DirEntry, File},
    io::{self, ErrorKind, Read},
    marker::PhantomData,
    path::PathBuf,
};

use crate::environment::EnvDiff;

pub trait Session: Sized {
    const LOOKUP_PATH: &str;
    const XDG_TYPE: &str;

    fn env(self) -> EnvDiff;
}

// TODO: implement display
pub struct SessionMetadata<T: Session> {
    _type: PhantomData<T>,
    name: String,
    comment: Option<String>,
    executable: PathBuf,
}

impl<T: Session> SessionMetadata<T> {
    fn parse_file(file: &mut File) -> Result<Self> {
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

        Ok(Self {
            _type: PhantomData,
            name: parsed.name.default,
            comment: parsed.comment.map(|x| x.default),
            executable,
        })
    }

    fn lookup(name: &str) -> Result<Self> {
        let path = PathBuf::from(T::LOOKUP_PATH).join(format!("{name}.desktop"));

        let mut file = File::open(path).map_err(|e| match e.kind() {
            ErrorKind::NotFound => anyhow!("Such a session is not defined"),
            _ => e.into(),
        })?;

        Self::parse_file(&mut file).context("Session definition is incorrect")
    }

    fn lookup_all() -> Vec<Self> {
        let dir = match PathBuf::from(T::LOOKUP_PATH).read_dir() {
            Ok(dir) => dir,
            Err(_) => return Vec::new(), // TODO: consider propagating error if it is not "path missing"
        };

        fn files(entry: io::Result<DirEntry>) -> Option<File> {
            // TODO: propagate errors
            let entry = entry.ok()?;
            if !entry.metadata().ok()?.is_file() {
                return None;
            }

            File::open(entry.path()).ok()
        }

        dir.filter_map(files)
            .filter_map(|mut file| Self::parse_file(&mut file).ok())
            .collect()
    }
}

// TODO: is setting this to the SessionMetadata.name appropriate?
// The spec says this can contain list of values
crate::define_env!("XDG_CURRENT_DESKTOP", SessionNameEnv(String));

crate::define_env!("XDG_SESSION_TYPE", SessionTypeEnv(String));

pub trait SessionManager<T: Session>: Sized {
    fn setup(self) -> Result<T>;

    fn start(self, session_name: &str) -> Result<()> {
        let metadata = SessionMetadata::<T>::lookup(session_name)?;
        let session_instance = self.setup()?;

        let env = EnvDiff::build()
            .set(SessionNameEnv(metadata.name))
            .set(SessionTypeEnv(T::XDG_TYPE.to_string()))
            .build()
            + session_instance.env();

        // TODO: spawn main executable

        Ok(())
    }
}

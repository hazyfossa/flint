use anyhow::{Context, Result, anyhow, bail};
use freedesktop_file_parser::{self as parser, EntryType};
use fs_err::{DirEntry, File, read_dir};
use serde::de::DeserializeOwned;

use std::{
    fmt::Display,
    io::{self, ErrorKind, Read},
    path::PathBuf,
    process::{self, Command},
};

use crate::{
    context::SessionContext,
    environment::{Env, EnvContainer, EnvRecipient},
    utils::config::Config,
};

pub struct SessionMetadata {
    name: String,
    comment: Option<String>,
    executable: PathBuf,
}

pub trait SessionMetadataLookup {
    fn lookup_metadata(name: &str) -> Result<SessionMetadata>;

    /// This function will return metadata for all available sessions
    /// It is currently not guaranteed that a session can be started for each entry
    /// I.e. the metadata can specify an executable that is unavailable.
    ///
    /// Entries with invalid metadata are silently discarded.
    fn lookup_metadata_all() -> Vec<SessionMetadata>;
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
        comment: parsed.comment.map(|x| x.default),
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

    fn lookup_metadata_all() -> Vec<SessionMetadata> {
        let dir = match read_dir(Self::LOOKUP_PATH) {
            Ok(dir) => dir,
            Err(_) => return Vec::new(),
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

impl Display for SessionMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)?;
        if let Some(comment) = &self.comment {
            write!(f, ": {}", comment)?;
        };
        writeln!(f, "")
    }
}

// TODO: is setting this to the SessionMetadata.name appropriate?
// The spec says this can contain list of values
crate::define_env!("XDG_CURRENT_DESKTOP", SessionNameEnv(String));

crate::define_env!("XDG_SESSION_TYPE", SessionTypeEnv(String));

pub trait SessionManager: Sized + Default + DeserializeOwned + SessionMetadataLookup {
    const XDG_TYPE: &str;
    type Env: EnvContainer;

    fn setup_session(&self, context: SessionContext) -> Result<Self::Env>;

    fn new_from_config(config: &Config) -> Result<Self> {
        let config = match config.session.get(Self::XDG_TYPE) {
            Some(config) => config.clone(), // TODO
            None => return Ok(Self::default()),
        };

        Ok(config.try_into()?)
    }

    fn new_session(
        self,
        metadata: SessionMetadata,
        context: SessionContext,
    ) -> Result<process::ExitStatus> {
        let session_instance_env = self.setup_session(context)?;

        let env = Env::empty()
            .set(SessionNameEnv(metadata.name))
            .set(SessionTypeEnv(Self::XDG_TYPE.to_string()))
            .set(session_instance_env);

        let mut command = Command::new(metadata.executable);
        let mut process = command
            .set_env(env)
            .spawn()
            .context("Failed to spawn main session process")?;

        Ok(process.wait().unwrap())
    }
}

#[macro_export]
macro_rules! sessions {
    ([$($session:ty),+]) => { // fn sessions([*session_types])
        $crate::scope!{($all:tt) => {
            #[macro_export]
            macro_rules! _dispatch_session {
                ($xdg_type:expr => $function:ident($all($args:tt)*)) => { // string => function(*arguments)
                    match $xdg_type {
                        // for T in session_types:
                        //     T::XDG_TYPE => function::<T>(*arguments)
                        $( <$session>::XDG_TYPE => $function::<$session>($all($args)*), )+
                        //
                        other => anyhow::bail!("{other} is not a valid session type."),
                    }
                }
            }
            pub use _dispatch_session as dispatch_session; // return _dispatch_session
        }}
    }
}

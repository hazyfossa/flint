use anyhow::{Context, Result, anyhow, bail};
use freedesktop_file_parser::{self as parser, EntryType};

use std::{
    fs::{DirEntry, File},
    io::{self, ErrorKind, Read},
    os::unix::process::CommandExt,
    path::PathBuf,
    process::{Child, Command},
};

use crate::{
    environment::{CommandEnvContextExt, EnvBundle, EnvContext, EnvValue},
    utils::subprocess::{ExitStatusExt, Ret},
};

// TODO: if i get to fully typesafe env changes, specify session-specific env types here
// pub trait SessionType<Env: EnvBundle>: Sized {
pub trait SessionType {
    const LOOKUP_PATH: &str;
    const XDG_TYPE: &str;

    fn lookup(name: &str) -> Result<Session> {
        let metadata = SessionMetadata::lookup(Self::LOOKUP_PATH.into(), name)?;
        Ok(Session {
            metadata,
            xdg_type: Self::XDG_TYPE,
        })
    }

    fn get_all() -> Option<Vec<Session>> {
        let path: PathBuf = Self::LOOKUP_PATH.into();
        let dir = path.read_dir().ok()?;

        fn files(entry: io::Result<DirEntry>) -> Option<File> {
            // TODO: propagate errors
            let entry = entry.ok()?;
            if !entry.metadata().ok()?.is_file() {
                return None;
            }

            File::open(entry.path()).ok()
        }

        Some(
            dir.filter_map(files)
                .filter_map(|mut file| SessionMetadata::parse_file(&mut file).ok())
                .map(|metadata| Session {
                    metadata,
                    xdg_type: Self::XDG_TYPE,
                })
                .collect(),
        )
    }
}

// TODO: implement display
pub struct SessionMetadata {
    name: String,
    comment: Option<String>,
    executable: PathBuf,
}

impl SessionMetadata {
    fn lookup(lookup_path: PathBuf, name: &str) -> Result<Self> {
        let path = lookup_path.join(format!("{name}.desktop"));

        let mut file = File::open(path).map_err(|e| match e.kind() {
            ErrorKind::NotFound => anyhow!("Such a session is not defined"),
            _ => e.into(),
        })?;

        Self::parse_file(&mut file).context("Session definition is incorrect")
    }

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
            name: parsed.name.default,
            comment: parsed.comment.map(|x| x.default),
            executable,
        })
    }
}

struct SessionNameEnv(String);

impl EnvValue for SessionNameEnv {
    const KEY: &str = "XDG_CURRENT_DESKTOP";
    crate::env_impl!();
}

struct SessionTypeEnv(String);

impl EnvValue for SessionTypeEnv {
    const KEY: &str = "XDG_SESSION_TYPE";
    crate::env_impl!();
}

pub struct Session {
    xdg_type: &'static str,
    pub metadata: SessionMetadata,
}

impl EnvBundle for Session {
    fn apply(self, ctx: &mut EnvContext) {
        ctx.set(SessionNameEnv(self.metadata.name))
            .set(SessionTypeEnv(self.xdg_type.to_string()));
    }
}

pub struct DesktopRunner {
    main_executable: PathBuf,
    env: EnvContext,
}

impl DesktopRunner {
    pub fn new(session: Session, inherit_env: EnvContext) -> Self {
        let main_executable = session.metadata.executable.clone();

        let mut env = inherit_env;
        env.apply_bundle(session);

        Self {
            main_executable,
            env,
        }
    }

    pub fn start_client(&self, command: &mut Command) -> io::Result<Child> {
        command
            .with_env_context(self.env.clone())
            // .process_group(0)
            .spawn()
    }

    pub fn start_main(self) -> Result<Ret> {
        dbg!("start main");
        let mut child = self
            .start_client(&mut Command::new(self.main_executable.clone()))
            .context("Failed to start main session process")?;

        Ok(child.wait().unwrap().context_process_name(format!(
            "{:#?} (main desktop process)",
            self.main_executable
        )))
    }
}

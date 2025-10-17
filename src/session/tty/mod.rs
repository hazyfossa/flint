#![allow(dead_code)] // TODO

mod control;

use std::{
    os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd},
    path::PathBuf,
    process::Command,
};

use anyhow::{Context, Result, anyhow};
use command_fds::{CommandFdExt, FdMapping};
use rustix::fs::{Mode, OFlags};

use super::manager::prelude::*;

pub use crate::session::context::VtNumber;
use crate::session::tty::control::VTAccessor;

impl VtNumber {
    fn as_tty_string(&self) -> String {
        format!("tty{}", self.to_string())
    }
}

unsafe fn clone_fd<'a>(fd: BorrowedFd<'a>) -> OwnedFd {
    unsafe { OwnedFd::from_raw_fd(fd.as_raw_fd()) }
}

pub struct ActiveVT {
    settings: VTAccessor,
    number: VtNumber,
}

impl ActiveVT {
    fn from_fd(fd: OwnedFd, number: Option<VtNumber>) -> Result<Self> {
        let settings = VTAccessor::from_fd(fd)?;

        let number = number.unwrap_or(
            settings
                .get_common_state()
                .context("Failed to query active VT state to get the number")?
                .active_number
                .into(),
        );

        Ok(Self { settings, number })
    }

    pub fn open(number: VtNumber) -> Result<Self> {
        let fd = rustix::fs::open(
            format!("/dev/tty{}", number.to_string()),
            OFlags::RDWR | OFlags::NOCTTY,
            Mode::from_bits_truncate(0o666),
        )
        .context(format!("Failed to open tty {}", number.to_string()))?;

        Self::from_fd(fd, Some(number))
    }

    pub fn current(number: Option<VtNumber>) -> Result<Self> {
        // TODO: this means that self.descriptor will be closed on drop.
        // Is this appropriate for stdin?
        let stdin = unsafe { OwnedFd::from_raw_fd(0) };
        Self::from_fd(stdin, number)
    }

    pub fn bind<'a>(&self, command: &'a mut Command) -> Result<&'a mut Command> {
        // TODO: consider moving this logic to session leader child
        command.fd_mappings(
            [0, 1, 2]
                .iter()
                // TODO: safety
                .map(|i| unsafe {
                    FdMapping {
                        parent_fd: clone_fd(self.settings.as_fd()),
                        child_fd: *i,
                    }
                })
                .collect(),
        )?;
        // TODO: set as controlling tty
        Ok(command)
    }
}

pub struct Session;

#[derive(Default, Deserialize)]
#[serde(default)]
pub struct Config {}

impl manager::SessionType for Session {
    const XDG_TYPE: &str = "tty";

    type ManagerConfig = Config;
    type EnvDiff = VtNumber;

    fn setup_session(_config: &Config, context: SessionContext) -> Result<Self::EnvDiff> {
        Ok(context.vt.clone())
    }

    fn spawn_session(
        _config: &Config,
        _executable: PathBuf,
        _context: SessionContext,
    ) -> Result<std::process::Child> {
        todo!()
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
                name: "Shell".to_string(),
                description: Some("Default shell as set for the target user".to_string()),
                executable: PathBuf::from("<set_externally>"),
            },
        )
    }
}

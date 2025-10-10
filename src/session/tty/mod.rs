#![allow(dead_code)] // TODO

mod ioctl;

use std::{
    io::IsTerminal,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    process::Command,
};

use anyhow::{Context, Result, bail};
use command_fds::{CommandFdExt, FdMapping};
use rustix::fs::{Mode, OFlags};

use super::manager::prelude::*;
use crate::session::manager::SessionMetadataLookup;

pub use crate::session::context::VtNumber;

impl VtNumber {
    fn as_tty_string(&self) -> String {
        format!("tty{}", self.to_string())
    }
}

unsafe fn clone_fd(fd: &OwnedFd) -> OwnedFd {
    unsafe { OwnedFd::from_raw_fd(fd.as_raw_fd()) }
}

pub struct VT {
    descriptor: OwnedFd,
}

impl VT {
    pub fn open(number: VtNumber) -> Result<Self> {
        let number = number.to_string();

        let fd = rustix::fs::open(
            format!("/dev/tty{number}"),
            OFlags::RDWR | OFlags::NOCTTY,
            Mode::from_bits_truncate(0o666),
        )
        .context(format!("Failed to open tty {number}"))?;

        Self::from_fd(fd)
    }

    pub fn from_fd(fd: OwnedFd) -> Result<Self> {
        if !fd.is_terminal() {
            bail!("descriptor is not a terminal")
        };
        Ok(Self { descriptor: fd })
    }

    pub fn try_from_stdin() -> Option<Self> {
        // TODO: this means that self.descriptor will be closed on drop.
        // Is this appropriate for stdin?
        let stdin = unsafe { OwnedFd::from_raw_fd(0) };
        match Self::from_fd(stdin) {
            Ok(terminal) => Some(terminal),
            Err(_) => None,
        }
    }

    pub fn bind<'a>(&self, command: &'a mut Command) -> Result<&'a mut Command> {
        // TODO: consider moving this logic to session leader child
        command.fd_mappings(
            [0, 1, 2]
                .iter()
                // TODO: safety
                .map(|i| unsafe {
                    FdMapping {
                        parent_fd: clone_fd(&self.descriptor),
                        child_fd: *i,
                    }
                })
                .collect(),
        )?;
        // TODO: set as controlling tty
        Ok(command)
    }
}

#[derive(Default, Deserialize)]
pub struct SessionManager {}

impl manager::SessionManager for SessionManager {
    const XDG_TYPE: &str = "tty";

    type Env = VtNumber;

    fn new_session(
        self,
        _metadata: manager::SessionMetadata,
        _context: SessionContext,
    ) -> Result<std::process::ExitStatus> {
        todo!()
    }
}

impl SessionMetadataLookup for SessionManager {
    fn lookup_metadata(_name: &str) -> Result<manager::SessionMetadata> {
        // name here is an executable
        todo!()
    }

    fn lookup_metadata_all() -> Vec<manager::SessionMetadata> {
        // TODO: at least debian provides a list of shells
        Vec::new()
    }
}

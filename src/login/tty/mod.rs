#![allow(dead_code)] // TODO

pub mod control;

use std::{
    os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd},
    process::Command,
};

use anyhow::{Context, Result};
use command_fds::{CommandFdExt, FdMapping};
use rustix::fs::{Mode, OFlags};

pub use crate::login::context::VtNumber;
use control::VTAccessor;

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

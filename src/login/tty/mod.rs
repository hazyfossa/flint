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
use control::{RenderMode, VTAccessor, activate};

impl VtNumber {
    fn to_tty_string(&self) -> String {
        format!("tty{}", self.to_string())
    }
}

unsafe fn clone_fd<'a>(fd: BorrowedFd<'a>) -> OwnedFd {
    unsafe { OwnedFd::from_raw_fd(fd.as_raw_fd()) }
}

pub struct ActiveVT {
    settings: VTAccessor,
    pub number: VtNumber,
}

impl ActiveVT {
    // This function will do everything required for the provided `fd`
    // to become an active VT under `number`
    // including switching away from the currently active VT
    //
    // Can cause screen flicker
    pub fn new(number: VtNumber, render_mode: RenderMode) -> Result<Self> {
        let fd = rustix::fs::open(
            format!("/dev/{}", number.to_tty_string()),
            OFlags::RDWR | OFlags::NOCTTY,
            Mode::from_bits_truncate(0o666),
        )
        .context(format!("Failed to open tty {}", number.to_string()))?;

        let accessor = VTAccessor::from_fd(fd)?;

        accessor
            .set_render_mode(render_mode)
            .context("failed to set VT render mode")?;

        accessor.clear().context("Failed to clear terminal")?;

        let currently_active: VtNumber = accessor
            .get_common_state()
            .context("Failed to query active VT state")?
            .active_number
            .into();

        if currently_active != number {
            // TODO: is changing general mode from default (None) useful?
            activate(&accessor, number, None).context("Failed to activate VT")?;
        }

        Ok(Self {
            settings: accessor,
            number,
        })
    }

    pub fn current(number: VtNumber) -> Result<Self> {
        // TODO: this means that self.descriptor will be closed on drop.
        // Is this appropriate for stdin?
        let stdin = unsafe { OwnedFd::from_raw_fd(0) };

        // TODO: is this enough? Should we activate just in case?

        Ok(Self {
            settings: VTAccessor::from_fd(stdin)?,
            number,
        })
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

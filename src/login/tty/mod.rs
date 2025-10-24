pub mod control;

use std::os::fd::{FromRawFd, OwnedFd};

use anyhow::{Context, Result};
use rustix::fs::{Mode, OFlags};

pub use crate::login::context::VtNumber;
use control::{RenderMode, VTAccessor};

impl VtNumber {
    pub fn to_tty_string(&self) -> String {
        format!("tty{}", self.to_string())
    }
}

pub struct ActiveVT {
    accessor: VTAccessor,
    pub number: VtNumber,
}

impl ActiveVT {
    pub fn new(number: VtNumber) -> Result<Self> {
        let fd = rustix::fs::open(
            format!("/dev/{}", number.to_tty_string()),
            OFlags::RDWR | OFlags::NOCTTY,
            Mode::from_bits_truncate(0o666),
        )
        .context(format!("Failed to open tty {}", number.to_string()))?;

        let accessor = VTAccessor::from_fd(fd)?;

        Ok(Self { accessor, number })
    }

    pub fn raw(self) -> VTAccessor {
        self.accessor
    }

    pub fn set_as_current(&self) -> Result<()> {
        self.accessor.bind_stdio().context("Failed to bind stdio")?;

        self.accessor
            .set_as_controlling_tty()
            .context("Failed to set as controlling tty")?;

        Ok(())
    }

    pub fn current(number: VtNumber) -> Result<Self> {
        // TODO: this means that self.descriptor will be closed on drop.
        // Is this appropriate for stdin?
        let stdin = unsafe { OwnedFd::from_raw_fd(0) };

        Ok(Self {
            accessor: VTAccessor::from_fd(stdin)?,
            number,
        })
    }

    // NOTE: can cause screen flicker due to VT switching
    // if another VT is active
    pub fn activate(&self, render_mode: RenderMode) -> Result<()> {
        self.accessor
            .set_render_mode(render_mode)
            .context("failed to set VT render mode")?;

        self.accessor.clear().context("Failed to clear terminal")?;

        let currently_active = VtNumber::manually_checked_from(
            self.accessor
                .get_common_state()
                .context("Failed to query active VT state")?
                .active_number,
        );

        if currently_active != self.number {
            // TODO: is changing general mode from default (None) useful?
            self.accessor
                .activate(self.number, None)
                .context("Failed to activate VT")?;
        }

        Ok(())
    }
}

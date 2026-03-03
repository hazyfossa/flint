pub mod control;

use std::os::fd::{FromRawFd, OwnedFd};

use anyhow::{Context, Result};
use rustix::fs::{Mode, OFlags};
use shrinkwraprs::Shrinkwrap;

pub use control::RenderMode as VtRenderMode;
use control::VTAccessor;

#[derive(Shrinkwrap, Clone, Copy, PartialEq)]
pub struct VtNumber(pub u16);

impl VtNumber {
    // This function is soft-unsafe, as it is the caller responsibility
    // to ensure "number" indicates a valid VT to handle
    //
    // For example, it is a really bad idea to assign this to an arbitrary value
    // as that will allow (among other things) switching to this VT while another program is running in it
    // While not undefined behaviour, this is undesirable.
    //
    // General rule of thumb: either the user or the kernel should tell you this VT number is free
    // before you call this
    pub fn manually_checked_from(number: u16) -> Self {
        Self(number)
    }

    pub fn to_tty_string(&self) -> String {
        format!("tty{}", self.to_string())
    }
}

pub struct Terminal {
    pub raw: VTAccessor,
    pub number: VtNumber,
}

impl Terminal {
    pub fn new(number: VtNumber) -> Result<Self> {
        let fd = rustix::fs::open(
            format!("/dev/{}", number.to_tty_string()),
            OFlags::RDWR | OFlags::NOCTTY,
            Mode::from_bits_truncate(0o666),
        )
        .context(format!("Failed to open tty {}", number.to_string()))?;

        let accessor = VTAccessor::from_fd(fd)?;

        Ok(Self {
            raw: accessor,
            number,
        })
    }

    pub fn set_as_current(&self) -> Result<()> {
        self.raw.bind_stdio().context("Failed to bind stdio")?;

        self.raw
            .set_as_controlling_tty()
            .context("Failed to set as controlling tty")?;

        Ok(())
    }

    pub fn current(number: VtNumber) -> Result<Self> {
        // TODO: this means that self.descriptor will be closed on drop.
        // Is this appropriate for stdin?
        let stdin = unsafe { OwnedFd::from_raw_fd(0) };

        Ok(Self {
            raw: VTAccessor::from_fd(stdin)?,
            number,
        })
    }

    // NOTE: can cause screen flicker due to VT switching
    // if another VT is active
    pub fn activate(&self, render_mode: VtRenderMode) -> Result<()> {
        self.raw
            .set_render_mode(render_mode)
            .context("failed to set VT render mode")?;

        self.raw.clear().context("Failed to clear terminal")?;

        let currently_active = VtNumber::manually_checked_from(
            self.raw
                .get_common_state()
                .context("Failed to query active VT state")?
                .active_number,
        );

        if currently_active != self.number {
            // TODO: is changing general mode from default (None) useful?
            self.raw
                .activate(self.number, None)
                .context("Failed to activate VT")?;
        }

        Ok(())
    }
}

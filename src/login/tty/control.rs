#![allow(dead_code)]

use std::{
    io::IsTerminal,
    os::fd::{AsFd, BorrowedFd, OwnedFd},
};

use anyhow::{Result, bail};
use rustix::{
    io::{self, write},
    ioctl, stdio,
};

// TODO: some OPs here are better defined as IntegerSetter, as they do not require a pointer

use super::VtNumber;

pub struct VTAccessor(OwnedFd);

impl VTAccessor {
    pub fn from_fd(fd: OwnedFd) -> Result<Self> {
        if !fd.is_terminal() {
            bail!("descriptor is not a terminal")
        };
        Ok(Self(fd))
    }
}

impl AsFd for VTAccessor {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

macro_rules! vt_property {
    (
        $model:ty,
        get = $opcode_get:expr
        $(, set = $opcode_set:expr)?

    ) => {
        paste::paste! {
        impl VTAccessor {
            pub fn [<get_ $model:snake>](&self) -> io::Result<$model> {
                unsafe { ioctl::ioctl(&self.0, ioctl::Getter::<$opcode_get, $model>::new()) }
            }

            $(pub fn [<set_ $model:snake>](&self, value: $model) -> io::Result<()> {
                unsafe {
                    ioctl::ioctl(
                        &self.0,
                        ioctl::IntegerSetter::<$opcode_set>::new_usize(value as _)
                    )
                }
            })?
        }
    }};
}

// State

#[repr(C)]
pub struct CommonState {
    pub active_number: u16,
    pub signal: u16,
    pub state: u16,
}

vt_property!(CommonState, get = 0x5603);

// Render mode

#[repr(i32)]
pub enum RenderMode {
    Text = 0,
    Graphics = 1,
}

vt_property!(RenderMode, get = 0x4B3B, set = 0x4B3A);

// Keyboard

#[repr(i32)]
#[derive(Debug)]
pub enum KeyboardMode {
    Disabled = 4,
    Scancode = 0,
    Keycode = 2,
    Ascii = 1,
    Unicode = 3,
}

vt_property!(KeyboardMode, get = 0x4B44, set = 0x4B45);

// VT Mode

// TODO
#[repr(u8)]
pub enum SwitchMode {
    Auto,    // auto vt switching
    Process, // process controls switching
    AckAcq,  // acknowledge switch
}

#[repr(C)]
pub struct Mode {
    pub switch_mode: SwitchMode,
    pub wait_on_write_to_inactive: u8,
    pub signal_release: u16,
    pub signal_acquire: u16,
    pub _unused: u16,
}

impl Default for Mode {
    fn default() -> Self {
        Self {
            switch_mode: SwitchMode::Auto,
            wait_on_write_to_inactive: 0,
            signal_release: 0,
            signal_acquire: 0,
            _unused: 0,
        }
    }
}

// Activate

type IoSetActivateVT = ioctl::Setter<0x560F, SwitchVtTarget>;
type IoWaitVT = ioctl::Setter<0x5607, u16>;

struct SwitchVtTarget {
    number: u64,
    mode: Mode,
}

impl VTAccessor {
    pub fn activate(&self, number: VtNumber, mode: Option<Mode>) -> io::Result<()> {
        let target = SwitchVtTarget {
            number: *number as _,
            mode: mode.unwrap_or_default(),
        };

        unsafe {
            ioctl::ioctl(&self.0, IoSetActivateVT::new(target))?;
            ioctl::ioctl(&self.0, IoWaitVT::new(*number))?;
        };

        Ok(())
    }
}

// Set as controlling tty

type IoSetCtty = ioctl::IntegerSetter<0x540E>;

impl VTAccessor {
    pub fn set_as_controlling_tty(&self) -> io::Result<()> {
        unsafe { ioctl::ioctl(&self.0, IoSetCtty::new_usize(1)) }
    }
}

// Other

impl VTAccessor {
    pub fn clear(&self) -> io::Result<()> {
        write(&self.0, b"\x1B[H\x1B[2J")?;
        Ok(())
    }

    pub fn bind_stdio(&self) -> io::Result<()> {
        stdio::dup2_stdin(&self.0)?;
        stdio::dup2_stdout(&self.0)?;
        stdio::dup2_stderr(&self.0)?;
        Ok(())
    }
}

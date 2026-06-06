use std::{
    ops::Deref,
    os::fd::{AsFd, BorrowedFd, OwnedFd},
};

use anyhow::{Context, Result, bail, ensure};
use rustix::{
    fs::{self, OFlags},
    io, ioctl, stdio,
};

// TODO: for cases when we immediately set as ctty via ioctl, not setting NOCTTY is an optimization.
fn open_dev(name: &str) -> io::Result<OwnedFd> {
    rustix::fs::open(
        format!("/dev/{}", name),
        OFlags::RDWR | OFlags::NOCTTY,
        fs::Mode::from_bits_truncate(0o666),
    )
}

// Terminal is a linux tty
// It can refer to both serial consoles and VTs
pub struct Terminal<F> {
    fd: F,
    pub number: u8,
}

impl<F: AsFd> Terminal<F> {
    pub fn try_from_fd(fd: F) -> Result<Self> {
        let stat = fs::fstat(&fd).context("fstat() failed")?;

        ensure!(stat.st_mode & fs::FileType::CharacterDevice.as_raw_mode() == 0);

        let (major, minor) = (fs::major(stat.st_rdev), fs::minor(stat.st_rdev));

        // 4 is type: tty
        ensure!(major == 4);

        // minor device number == tty number
        let number = minor.try_into().context("Invalid minor device number")?;

        Ok(Self { fd, number })
    }

    pub fn clear(&self) -> io::Result<()> {
        rustix::io::write(&self.fd, b"\x1B[H\x1B[2J")?;
        Ok(())
    }

    pub fn set_as_ctty(&self) -> io::Result<()> {
        type I = ioctl::IntegerSetter<0x540E>;
        // Safety: self.fd is a terminal
        unsafe { ioctl::ioctl(&self.fd, I::new_usize(1)) }
    }

    pub fn set_as_stdio(&self) -> io::Result<()> {
        let fd = self.fd.as_fd();
        stdio::dup2_stdin(&fd)?;
        stdio::dup2_stdout(&fd)?;
        stdio::dup2_stderr(&fd)?;
        Ok(())
    }

    pub fn try_as_vt(self) -> Result<VT<F>> {
        if self.number >= 64 {
            bail!("tty is not a vt")
        }

        Ok(VT { terminal: self })
    }
}

// TODO: figure out if we need this (only case i see is serial console support)
// impl Terminal<OwnedFd> {
//     // The controlling terminal of the current session.
//     pub fn current_ctty() -> Result<Self> {
//         let fd = open_dev("tty").context("Failed to open current ctty")?;

//         Ok(Self { fd })
//     }
// }

impl Terminal<BorrowedFd<'static>> {
    pub fn current_io() -> Option<Self> {
        Self::try_from_fd(stdio::stdin())
            .or_else(|_| Self::try_from_fd(stdio::stdout()))
            .or_else(|_| Self::try_from_fd(stdio::stderr()))
            .ok()
    }
}

pub struct VtNumber(u8);

impl Deref for VtNumber {
    type Target = u8;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl VtNumber {
    pub fn new(i: u8) -> Option<Self> {
        if i >= 64 {
            return None;
        } else {
            return Some(Self(i));
        }
    }
}

pub struct VT<F> {
    terminal: Terminal<F>,
}

impl<F> Deref for VT<F> {
    type Target = Terminal<F>;
    fn deref(&self) -> &Self::Target {
        &self.terminal
    }
}

impl VT<OwnedFd> {
    pub fn open(number: VtNumber) -> Result<Self> {
        let number = *number;

        let fd = rustix::fs::open(
            format!("/dev/tty{}", number),
            OFlags::RDWR | OFlags::NOCTTY,
            fs::Mode::from_bits_truncate(0o666),
        )
        .context(format!("Failed to open tty for vt {}", number))?;

        let terminal = Terminal { fd, number };

        Ok(Self { terminal })
    }

    pub fn number(&self) -> VtNumber {
        VtNumber(self.terminal.number)
    }

    pub fn current_active() -> Result<Self> {
        let fd = open_dev("tty0").context("Failed to open current VT")?;
        let terminal = Terminal::try_from_fd(fd)?;
        Ok(Self { terminal })
    }
}

macro_rules! vt_property {
    (
        $model:ty,
        get = $opcode_get:expr
        $(, set = $opcode_set:expr)?

    ) => {
        paste::paste! {
        impl<F: AsFd> VT<F> {
            pub fn [<get_ $model:snake>](&self) -> io::Result<$model> {
                unsafe { ioctl::ioctl(&self.fd, ioctl::Getter::<$opcode_get, $model>::new()) }
            }

            $(pub fn [<set_ $model:snake>](&self, value: $model) -> io::Result<()> {
                unsafe {
                    ioctl::ioctl(
                        &self.fd,
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

struct SwitchVtTarget {
    number: u64,
    mode: Mode,
}

impl<F: AsFd> VT<F> {
    pub(super) fn activate(&self) -> io::Result<()> {
        let target = SwitchVtTarget {
            number: self.number as _,

            // we currently do not plan to support
            // process-controlled (exclusive) switching
            mode: Mode::default(),
        };

        type IoSetActivateVT = ioctl::Setter<0x560F, SwitchVtTarget>;
        type IoWaitVT = ioctl::Setter<0x5607, u16>;

        unsafe {
            ioctl::ioctl(&self.fd, IoSetActivateVT::new(target))?;
            ioctl::ioctl(&self.fd, IoWaitVT::new(self.number.into()))?;
        };

        Ok(())
    }
}

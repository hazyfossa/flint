mod keyboard;

use std::{
    fs::File,
    io::IsTerminal,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
};

use anyhow::{Context, Result, bail};
use command_fds::FdMapping;
use rustix::ioctl;

crate::define_env!("XDG_VTNR", pub VtNumber(u8));

unsafe fn unsafe_clone_fd(fd: &OwnedFd) -> OwnedFd {
    unsafe { OwnedFd::from_raw_fd(fd.as_raw_fd()) }
}

type IoChangeVt = ioctl::Setter<0x5606, VtNumber>;
type IoWaitVT = ioctl::Setter<0x5607, VtNumber>;

pub struct VT {
    descriptor: OwnedFd,
}

impl VT {
    pub fn open(number: VtNumber) -> Result<Self> {
        let number = number.to_string();
        let file = File::open(format!("/dev/tty{number}"))
            .context(format!("Failed to open tty {number}"))?;

        if !file.is_terminal() {
            bail!("Failed to open tty {number}: not a terminal")
        }

        Ok(Self {
            descriptor: file.into(),
        })
    }

    pub fn change_to(&self, number: VtNumber) -> Result<()> {
        // TODO
        unsafe {
            ioctl::ioctl(&self.descriptor, IoChangeVt::new(number.clone()))?;
            ioctl::ioctl(&self.descriptor, IoWaitVT::new(number))?;
        }

        Ok(())
    }

    pub fn stdio_bind_mappings<'a>(&self) -> Vec<FdMapping> {
        [0, 1, 2]
            .iter()
            // TODO: safety
            .map(|i| unsafe {
                FdMapping {
                    parent_fd: unsafe_clone_fd(&self.descriptor),
                    child_fd: *i,
                }
            })
            .collect()
    }
}

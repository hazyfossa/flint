use std::fs::File;
use std::io::Write;

use anyhow::{Context, Result};
use rustix::rand::{GetRandomFlags, getrandom};
use shrinkwraprs::Shrinkwrap;

#[derive(Shrinkwrap)]
pub struct Cookie(String);

impl Cookie {
    const RANDOM_BYTES: usize = 128;

    pub fn build() -> Result<Self> {
        let mut random = vec![0; Self::RANDOM_BYTES];
        getrandom(&mut random, GetRandomFlags::empty()).context("getrandom syscall failed")?;

        let hash = md5::compute(random);
        Ok(Self(format!("{:x}", hash)))
    }

    pub fn store(self, file: &mut File) -> Result<()> {
        file.write_all(self.as_bytes())
            .context("failed to write cookie to file")
    }
}

#[repr(u16)]
enum Family {
    Local = 256,
    Wild = 65535,
    Netname = 254,
    Krb5Principal = 253,
    LocalHost = 252,
}

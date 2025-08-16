use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use libxauth::{Cookie, XAuthorityFile, utils::LocalAuthorityBuilder};
use rustix::{
    rand::{GetRandomFlags, getrandom},
    system::uname,
};

use super::Display;
use crate::{console::VtNumber, environment::EnvValue};

pub struct ClientAuthorityEnv(OsString);

impl EnvValue for ClientAuthorityEnv {
    const KEY: &str = "XAUTHORITY";

    fn serialize(&self) -> OsString {
        self.0.clone()
    }

    fn deserialize(value: OsString) -> Result<Self> {
        Ok(Self(value))
    }
}

fn make_cookie() -> Result<Cookie> {
    let mut cookie_buf = [0u8; Cookie::BYTES_LEN];
    getrandom(&mut cookie_buf, GetRandomFlags::empty()).context("getrandom() failed")?;
    Ok(Cookie::new(cookie_buf))
}

fn get_hostname() -> Vec<u8> {
    uname().nodename().to_bytes().to_vec()
}

// TODO: allow for easy unlocked mode
pub struct XAuthorityManager {
    directory: PathBuf,
    builder: LocalAuthorityBuilder,
}

impl XAuthorityManager {
    pub fn new(directory: &Path) -> Result<Self> {
        let cookie = make_cookie()?;
        let hostname = get_hostname();

        Ok(Self {
            directory: directory.into(),
            builder: LocalAuthorityBuilder::new(cookie, hostname),
        })
    }

    pub fn setup_server(&self, vt: &VtNumber) -> Result<PathBuf> {
        let authority = self.builder.build_server().finish(); // TODO: here maybe transfer existing

        let path = self.directory.join(format!("vt-{vt}-authority"));

        let mut xauth_file =
            XAuthorityFile::create(&path).context(format!("Failed to create {path:?}"))?;
        xauth_file.set(authority)?;

        Ok(path)
    }

    // TODO: for multi-user support, make this return a ClientAuthBinder
    pub fn setup_client(self, display: &Display) -> Result<ClientAuthorityEnv> {
        let authority = self.builder.client(display.number().to_string());

        let path = self.directory.join("x-client-authority");

        let mut xauth_file =
            XAuthorityFile::create(&path).context(format!("Failed to create {path:?}"))?;
        xauth_file.set(authority)?;

        Ok(ClientAuthorityEnv(path.into()))
    }
}

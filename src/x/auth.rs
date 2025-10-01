use std::{
    ffi::OsString,
    fs::DirBuilder,
    io,
    os::unix::fs::DirBuilderExt,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use libxauth::*;
use rustix::{
    rand::{GetRandomFlags, getrandom},
    system::uname,
};

use super::Display;
use crate::{environment::EnvValue, tty::VtNumber, utils::runtime_dir};

pub struct ClientAuthorityEnv(OsString);

impl EnvValue for ClientAuthorityEnv {
    const KEY: &str = "XAUTHORITY";

    fn serialize(self) -> OsString {
        self.0
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

fn get_hostname() -> Hostname {
    uname().nodename().to_bytes().to_vec()
}

// TODO: is there anything we should do when hostname changes?
// Session should stay alive as clients fallback to local
// Are there any side-effects? What breaks?
pub struct XAuthorityManager {
    lock: bool,
    directory: PathBuf,
    cookie: Cookie,
    hostname: Hostname,
}

impl XAuthorityManager {
    pub fn new(vt: &VtNumber, lock: bool) -> Result<Self> {
        let cookie = make_cookie()?;
        let hostname = get_hostname();

        let runtime_dir = runtime_dir::current;
        let directory = runtime_dir.get()?.join(format!("vt-{}", vt.to_string()));

        // TODO: what to do with dir on Xorg exit?

        DirBuilder::new()
            .mode(0o700)
            .create(&directory)
            .context(format!("Failed to create {directory:?}"))?;

        Ok(Self {
            lock,
            directory,
            cookie,
            hostname,
        })
    }

    fn create_auth_file(&self, path: &Path) -> io::Result<AuthorityFile> {
        if self.lock {
            AuthorityFile::create(path)
        } else {
            // Safety: setting lock=false means user explicitly guarantees no other
            // party will interact with runtime dir on setup
            // TODO: maybe propagate safety of lock option better
            unsafe { AuthorityFile::create_unlocked(path) }
        }
    }

    pub fn setup_server(&self) -> Result<PathBuf> {
        let authority = Authority::new(Some(vec![Entry::new(
            &self.cookie,
            Scope::Any,
            Target::Server { slot: 0 },
        )]));

        let path = self.directory.join("server-authority");

        let mut xauth_file = self
            .create_auth_file(&path)
            .context(format!("Failed to create {path:?}"))?;

        xauth_file.set(authority)?;

        Ok(path)
    }

    pub fn setup_client(&self, display: &Display) -> Result<ClientAuthorityEnv> {
        let display_number = display.number().to_string();

        // TODO: add proper note why we do two entries
        // (legacy apps + hostname changes)

        let authority = Authority::new(Some(vec![
            Entry::new(
                &self.cookie,
                Scope::Any,
                Target::Client {
                    display_number: display_number.clone(),
                },
            ),
            Entry::new(
                &self.cookie,
                Scope::Local(self.hostname.clone()),
                Target::Client { display_number },
            ),
        ]));

        let path = self.directory.join("client-authority");

        let mut xauth_file = self
            .create_auth_file(&path)
            .context(format!("Failed to create {path:?}"))?;

        xauth_file.set(authority)?;

        Ok(ClientAuthorityEnv(path.into()))
    }

    // fn seal(self) {
    //     todo!()
    // }
}

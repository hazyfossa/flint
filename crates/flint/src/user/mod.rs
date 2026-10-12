#[cfg(feature = "userdb")]
mod userdb;

#[cfg(feature = "nss")]
mod nss;

use std::ffi::c_uint;

use eyre::Result;
use plug::plug;

pub type Uid = c_uint;
pub type Gid = c_uint;

// TODO: expand this
// TODO: optional exts (i.e. profile pictures)
pub struct UserMeta {
    pub uid: Uid,
    pub gid: Gid,
    pub home: String,
    pub shell: String,
}

mod nss;
mod userdb;

#[plug(userdb::UserDB, nss::NSS)]
pub trait UserProvider {
    async fn resolve(&mut self, name: &str) -> Result<Option<UserMeta>>;
}

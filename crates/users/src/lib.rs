#[cfg(feature = "nss")]
pub mod nss;

#[cfg(feature = "userdb")]
pub mod userdb;

use std::{
    ffi::{OsString, c_uint},
    path::PathBuf,
};

pub type Uid = c_uint;
pub type Gid = c_uint;

// TODO: expand this
// TODO: optional exts (i.e. profile pictures)
pub struct UserMeta {
    pub uid: Uid,
    pub gid: Gid,
    pub home: PathBuf,
    pub shell: OsString,
    // TODO: support `locked` (sp_expire)?
}

#[allow(async_fn_in_trait)]
pub trait UserProvider {
    type Error: std::error::Error;
    async fn resolve(&mut self, name: &str) -> Result<Option<UserMeta>, Self::Error>;
}

#[cfg(feature = "userdb")]
mod userdb;

#[cfg(feature = "nss")]
mod nss;

use std::{
    ffi::{OsString, c_uint},
    path::PathBuf,
};

use anyhow::Result;
use dyn_utils::{dyn_object, dyn_trait};

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

#[dyn_trait]
#[dyn_trait(dyn_object)]
pub trait UserProvider {
    #[dyn_trait(maybe_sync)]
    async fn resolve(&mut self, name: &str) -> Result<Option<UserMeta>>;
}

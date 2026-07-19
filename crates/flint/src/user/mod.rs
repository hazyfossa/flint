#[cfg(feature = "userdb")]
mod userdb;

#[cfg(feature = "nss")]
mod nss {
    pub struct NSS;

    impl super::UserProvider for NSS {
        async fn resolve(&mut self, name: &str) -> anyhow::Result<Option<super::UserMeta>> {
            Ok(flint_nss::resolve(name)?.map(|f| super::UserMeta {
                uid: f.uid,
                gid: f.gid,
                home: f.home,
                shell: f.shell,
            }))
        }
    }
}

use std::{
    ffi::{OsString, c_uint},
    path::PathBuf,
};

use anyhow::Result;
use dyn_utils::dyn_trait;

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
pub trait UserProvider {
    async fn resolve(&mut self, name: &str) -> Result<Option<UserMeta>>;
}

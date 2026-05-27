#[cfg(feature = "nss")]
pub mod nss;

use std::ffi::c_uint;

pub type Uid = c_uint;
pub type Gid = c_uint;

// TODO: expand this
// TODO: optional exts (i.e. profile pictures)
pub struct UserMeta {
    pub uid: Uid,
    pub gid: Gid,
}

#[allow(async_fn_in_trait)]
pub trait UserProvider {
    type Error: std::error::Error;
    async fn resolve(name: &str) -> Result<Option<UserMeta>, Self::Error>;
}

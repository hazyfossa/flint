// For implementation, see separate crate `flint-nss`

use super::*;
use anyhow::Result;
use dyn_utils::sync;

pub struct NSS;

impl UserProvider for NSS {
    #[sync]
    async fn resolve(&mut self, name: &str) -> Result<Option<UserMeta>> {
        Ok(flint_nss::resolve(name)?.map(|f| UserMeta {
            uid: f.uid,
            gid: f.gid,
            home: f.home,
            shell: f.shell,
        }))
    }
}

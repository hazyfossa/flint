// For implementation, see separate crate `flint-nss`

use super::*;
use eyre::Result;

#[plug]
pub struct NSS;

#[plug]
impl UserProvider::Interface for NSS {
    async fn resolve(&mut self, name: &str) -> Result<Option<UserMeta>> {
        todo!();
        // Ok(pwd_grp::getpwnam(name)?.map(|f| UserMeta {
        //     uid: f.uid,
        //     gid: f.gid,
        //     home: f.dir,
        //     shell: f.shell,
        // }))
    }
}

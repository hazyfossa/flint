use std::path::PathBuf;

use anyhow::Result;

use crate::environment::{Env, EnvContainer};

// NOTE: this is a stub
pub struct UserInfo {
    username: String,
    uid: u32,
    gid: u32,
    home: PathBuf,
}

pub struct UserID {
    pub uid: u32,
    pub gid: u32,
}

impl UserInfo {
    pub fn as_user_id(&self) -> UserID {
        UserID {
            uid: self.uid,
            gid: self.gid,
        }
    }
}

impl EnvContainer for UserInfo {
    fn apply_as_container(self, _env: Env) -> Env {
        todo!()
    }
}

pub trait UserInfoProvider {
    fn query(&self, name: &str) -> Result<UserInfo>;
}

use anyhow::Result;

pub mod env {
    use crate::environment::define_env;
    use std::path::PathBuf;

    define_env!(pub Home(PathBuf) = parse "HOME");
    define_env!(pub Shell(PathBuf) = parse "SHELL");
}

// NOTE: this is a stub
pub struct UserInfo {
    uid: u32,
    gid: u32,
    pub env: (env::Home, env::Shell),
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

pub trait UserInfoProvider {
    fn query(&self, name: &str) -> Result<UserInfo>;
}

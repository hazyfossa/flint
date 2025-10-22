use anyhow::Result;

use crate::environment::{Env, EnvContainer};

pub mod env {
    use std::path::PathBuf;

    use crate::environment::prelude::*;

    define_env!("HOME", pub Home(PathBuf));
    env_parser_raw!(Home);

    define_env!("SHELL", pub Shell(PathBuf));
    env_parser_raw!(Shell);
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

impl EnvContainer for UserInfo {
    fn apply_as_container(self, env: Env) -> Env {
        env.merge(self.env)
    }
}

pub trait UserInfoProvider {
    fn query(&self, name: &str) -> Result<UserInfo>;
}

use std::{os::unix::process::CommandExt, path::Path, process::Command};

use anyhow::{Context, Result};
use shrinkwraprs::Shrinkwrap;

use crate::{
    environment::{EnvContainer, prelude::*},
    login::UserInfo,
};

#[derive(Shrinkwrap, Clone)]
pub struct VtNumber(u16);

impl From<u16> for VtNumber {
    fn from(value: u16) -> Self {
        Self(value)
    }
}

impl_env!("XDG_VTNR", VtNumber);
env_parser_auto!(VtNumber);

define_env!("XDG_SEAT", pub Seat(String));
env_parser_auto!(Seat);

impl Default for Seat {
    fn default() -> Self {
        // man sd-login says that seat0 always exists
        Self("seat0".into())
    }
}

// UserIncomplete, Manager, Bacjground and None are not here as those aren't relevant
#[allow(dead_code)]
pub enum SessionClass {
    User { early: bool, light: bool },
    Greeter,
    LockScreen,
}

impl_env!("XDG_SESSION_CLASS", SessionClass);

impl EnvParser for SessionClass {
    fn serialize(&self) -> std::ffi::OsString {
        match *self {
            Self::User { early, light } => {
                let mut string = "user".to_string();
                if early {
                    string += "-early"
                }
                if light {
                    string += "-light"
                }
                string.into()
            }
            Self::Greeter => "greeter".into(),
            Self::LockScreen => "lock-screen".into(),
        }
    }

    fn deserialize(_value: std::ffi::OsString) -> Result<Self> {
        todo!()
    }
}

pub struct UserSwitch {
    uid: u32,
    gid: u32,
}

impl UserInfo {
    pub fn user_switch_data(&self) -> UserSwitch {
        UserSwitch {
            uid: self.uid,
            gid: self.gid,
        }
    }
}

pub type ExitReason = String;

pub struct LoginContext {
    pub vt: VtNumber,
    pub seat: Seat,

    pub env: Env,
    switch_user: Option<UserSwitch>,
}

impl LoginContext {
    pub(super) fn from_env(mut env: Env, switch_user: Option<UserSwitch>) -> Result<Self> {
        Ok(Self {
            vt: env.pull()?,
            seat: env.pull().unwrap_or_default(), // Propagate if seat exists but invalid
            env,
            switch_user,
        })
    }

    pub fn current(env: Env) -> Result<Self> {
        Self::from_env(env, None).context(
            "Cannot take control over current login context.
            Most likely you are already running a graphical session.",
        )
    }

    pub fn command(&self, program: &Path) -> Command {
        let mut cmd = Command::new(program);

        if let Some(switch_user) = &self.switch_user {
            // TODO: consider writing a manual impl instead of std
            cmd.uid(switch_user.uid).gid(switch_user.gid);
        };

        cmd
    }
}

impl EnvContainer for LoginContext {
    fn apply_as_container(self, env: Env) -> Env {
        env.merge(self.env)
    }
}

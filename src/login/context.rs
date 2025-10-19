use std::{os::unix::process::CommandExt, path::Path, process::Command};

use anyhow::{Context, Result};
use rustix::process::{self, Signal};
use shrinkwraprs::Shrinkwrap;

use crate::{
    environment::{EnvContainer, EnvRecipient, prelude::*},
    login::users::UserID,
    utils::runtime_dir::RuntimeDirManager,
};

#[derive(Shrinkwrap, Clone)]
pub struct VtNumber(u16);

impl From<u16> for VtNumber {
    fn from(value: u16) -> Self {
        Self(value)
    }
}

impl EnvVar for VtNumber {
    const KEY: &str = "XDG_VTNR";
}
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

impl EnvVar for SessionClass {
    const KEY: &str = "XDG_SESSION_CLASS";
}

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

pub type ExitReason = String;

pub struct LoginContext {
    pub vt: VtNumber,
    pub seat: Seat,

    pub env: Env,
    pub user: Option<UserID>,
    pub runtime_dir_manager: RuntimeDirManager,
}

impl LoginContext {
    pub(super) fn from_env(mut env: Env, switch_user: Option<UserID>) -> Result<Self> {
        let runtime_dir_manager =
            RuntimeDirManager::from_env(&env).context("Failed to create runtime dir manager")?;

        Ok(Self {
            vt: env.pull()?,
            seat: env.pull().unwrap_or_default(), // Propagate if seat exists but invalid
            env,
            user: switch_user,
            runtime_dir_manager,
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

        if let Some(switch_user) = &self.user {
            cmd.uid(switch_user.uid).gid(switch_user.gid);
        }

        unsafe {
            cmd.pre_exec(|| {
                process::set_parent_process_death_signal(Some(Signal::TERM))?;
                Ok(())
            });
        }

        cmd.set_env(self.env.clone()).unwrap();
        cmd
    }
}

impl EnvContainer for LoginContext {
    fn apply_as_container(self, env: Env) -> Env {
        env.merge(self.env)
    }
}

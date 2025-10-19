use std::{os::unix::process::CommandExt, path::Path, process::Command};

use anyhow::{Context, Result};
use rustix::process::{self, Signal};

use crate::{
    environment::{EnvContainer, EnvRecipient, prelude::*},
    login::{tty::ActiveVT, users::UserID},
    utils::runtime_dir::RuntimeDirManager,
};

#[derive(Clone, Copy, PartialEq)]
pub struct VtNumber(u16);

impl From<u16> for VtNumber {
    fn from(value: u16) -> Self {
        Self(value)
    }
}

impl VtNumber {
    pub fn as_int(self) -> u16 {
        self.0
    }
}

impl ToString for VtNumber {
    fn to_string(&self) -> String {
        self.0.to_string()
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
    pub terminal: ActiveVT,
    pub seat: Seat,

    pub env: Env,
    pub user: Option<UserID>,
    pub runtime_dir_manager: RuntimeDirManager,
}

impl LoginContext {
    pub fn new(env: Env, seat: Seat, terminal: ActiveVT, switch_user: UserID) -> Result<Self> {
        let runtime_dir_manager =
            RuntimeDirManager::from_env(&env).context("Failed to create runtime dir manager")?;

        Ok(Self {
            terminal,
            seat,
            env,
            user: Some(switch_user),
            runtime_dir_manager,
        })
    }

    pub fn current(mut env: Env) -> Result<Self> {
        let runtime_dir_manager =
            RuntimeDirManager::from_env(&env).context("Failed to create runtime dir manager")?;

        let vt_number = env.pull::<VtNumber>().context(
            "Cannot take over current login context.
        Most likely you are already running a graphical session.",
        )?;

        let terminal = ActiveVT::current(vt_number)?;

        let seat = env.pull::<Seat>().unwrap_or_default();

        Ok(Self {
            terminal,
            seat,
            env,
            user: None,
            runtime_dir_manager,
        })
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

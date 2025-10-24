use anyhow::{Context, Result};
use shrinkwraprs::Shrinkwrap;

use crate::{
    environment::{EnvContainer, prelude::*},
    login::{tty::Terminal, users::UserID},
    utils::runtime_dir::RuntimeDirManager,
};

#[derive(Shrinkwrap, Clone, Copy, PartialEq)]
pub struct VtNumber(u16);

impl VtNumber {
    // This function is soft-unsafe, as it is the caller responsibility
    // to ensure "number" indicates a valid VT to handle
    //
    // For example, it is a really bad idea to assign this to an arbitrary value
    // as that will allow (among other things) switching to this VT while another program is running in it
    // While not undefined behaviour, this is undesirable.
    //
    // General rule of thumb: either the user or the kernel should tell you this VT number is free
    // before you call this
    pub fn manually_checked_from(number: u16) -> Self {
        Self(number)
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

pub struct LoginContext {
    pub terminal: Option<Terminal>,
    pub seat: Seat,

    pub env: Env,

    pub user: Option<UserID>,
    pub runtime_dir_manager: RuntimeDirManager,
}

impl LoginContext {
    pub fn new(
        env: Env,
        seat: Seat,
        terminal: Option<Terminal>,
        switch_user: UserID,
    ) -> Result<Self> {
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

        let terminal =
            Terminal::current(vt_number).context("Cannot open current terminal (interactively)")?;

        let seat = env.pull::<Seat>().unwrap_or_default();

        Ok(Self {
            terminal: Some(terminal),
            seat,
            env,
            user: None,
            runtime_dir_manager,
        })
    }
}

impl EnvContainer for LoginContext {
    fn apply_as_container(self, env: Env) -> Env {
        env.merge(self.env)
    }
}

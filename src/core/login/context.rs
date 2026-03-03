use anyhow::{Context, Result};
use shrinkwraprs::Shrinkwrap;

use super::users::UserID;

use crate::{
    bind::tty::{VtNumber, VtRenderMode},
    environment::{Env, EnvironmentParse, EnvironmentVariable, define_env},
    utils::runtime_dir::RuntimeDirManager,
};

impl EnvironmentVariable for VtNumber {
    const KEY: &str = "XDG_VTNR";
}

impl EnvironmentParse<String> for VtNumber {
    fn env_serialize(self) -> String {
        self.0.to_string()
    }

    fn env_deserialize(raw: String) -> Result<Self> {
        Ok(Self(raw.parse()?))
    }
}

define_env!(pub Seat(String) = parse "XDG_SEAT");

impl Seat {
    pub fn into_id(self) -> String {
        self.0
    }
}

impl Default for Seat {
    fn default() -> Self {
        // man sd-login says that seat0 always exists
        Self("seat0".into())
    }
}

// UserIncomplete, Manager, Background and None are not here as those aren't relevant
#[allow(dead_code)]
pub enum SessionClass {
    User { early: bool, light: bool },
    Greeter,
    LockScreen,
}

impl EnvironmentVariable for SessionClass {
    const KEY: &str = "XDG_SESSION_CLASS";
}

impl EnvironmentParse<String> for SessionClass {
    fn env_serialize(self) -> String {
        match self {
            Self::User { early, light } => {
                let mut string = "user".to_string();
                if early {
                    string += "-early"
                }
                if light {
                    string += "-light"
                }
                string
            }
            Self::Greeter => "greeter".to_string(),
            Self::LockScreen => "lock-screen".to_string(),
        }
    }

    fn env_deserialize(_value: String) -> Result<Self> {
        todo!()
    }
}

pub struct LoginContext {
    pub vt: Option<VtNumber>,
    pub seat: Seat,

    pub env: Env,

    pub user: Option<UserID>,
    pub runtime_dir_manager: RuntimeDirManager,
}

impl LoginContext {
    pub fn new(env: Env, seat: Seat, vt: Option<VtNumber>, switch_user: UserID) -> Result<Self> {
        let runtime_dir_manager =
            RuntimeDirManager::from_env(&env).context("Failed to create runtime dir manager")?;

        Ok(Self {
            vt,
            seat,
            env,
            user: Some(switch_user),
            runtime_dir_manager,
        })
    }

    pub fn current(env: Env) -> Result<Self> {
        let runtime_dir_manager =
            RuntimeDirManager::from_env(&env).context("Failed to create runtime dir manager")?;

        // TODO: is this correct?
        let vt = env.get::<VtNumber>().context(
            "Cannot take over current login context.
        Most likely you are already running a graphical session.",
        )?;

        let seat = env.get::<Seat>().unwrap_or_default();

        Ok(Self {
            vt: Some(vt),
            seat,
            env,
            user: None,
            runtime_dir_manager,
        })
    }
}

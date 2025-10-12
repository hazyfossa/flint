use anyhow::{Context, Result};
use serde::{Deserialize, de::DeserializeOwned};

use std::{
    path::PathBuf,
    process::{self, Command},
};

use super::{context::SessionContext, metadata::SessionMetadataLookup};
use crate::{
    environment::{EnvContainer, EnvRecipient, EnvValue},
    session::metadata::{SessionMap, SessionMetadata},
    utils::{config::Config, misc::OsStringExt},
};

pub mod prelude {
    pub use crate::session::{
        context::SessionContext,
        manager,
        metadata::{self, SessionMap, SessionMetadata},
    };
    pub use serde::Deserialize;
}

crate::define_env!("XDG_SESSION_DESKTOP", SessionNameEnv(String));
crate::define_env!("XDG_SESSION_TYPE", SessionTypeEnv(String));

// TODO: investigate how this can contain more than one name
struct SessionCompositionEnv(Vec<String>);

impl SessionCompositionEnv {
    fn simple(name: String) -> Self {
        Self(vec![name])
    }
}

impl EnvValue for SessionCompositionEnv {
    const KEY: &str = "XDG_CURRENT_DESKTOP";

    fn serialize(self) -> std::ffi::OsString {
        self.0.join(";").into()
    }

    fn deserialize(value: std::ffi::OsString) -> Result<Self> {
        Ok(Self(
            value
                .try_to_string()?
                .split(';')
                .map(String::from)
                .collect(),
        ))
    }
}

// UserIncomplete, Manager, Bacjground and None are not here as those aren't relevant
#[allow(dead_code)]
pub enum SessionClass {
    User { early: bool, light: bool },
    Greeter,
    LockScreen,
}

impl EnvValue for SessionClass {
    const KEY: &str = "XDG_SESSION_CLASS";

    fn serialize(self) -> std::ffi::OsString {
        match self {
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

pub trait SessionType: Sized + SessionMetadataLookup {
    const XDG_TYPE: &str;

    type ManagerConfig: Default + DeserializeOwned;
    type EnvDiff: EnvContainer;

    fn setup_session(
        config: &Self::ManagerConfig,
        context: SessionContext,
    ) -> Result<Self::EnvDiff>;

    /// If you do not need fine-grained control over the startup flow
    /// consider implementing setup_session() instead
    fn spawn_session(
        config: &Self::ManagerConfig,
        executable: PathBuf,
        context: SessionContext,
    ) -> Result<process::Child> {
        // Note: this is a cheap clone
        // As Env is immutable underneath
        let env = context.env.clone();

        let session_env = Self::setup_session(config, context)?;

        Command::new(executable)
            .set_env(env.merge(session_env))
            .spawn()
            .context("Failed to spawn main session process")
    }
}

#[derive(Deserialize)]
pub struct SessionManager<T: SessionType> {
    #[serde(flatten)]
    config: T::ManagerConfig,
    entries: SessionMap,
}

impl<T: SessionType> SessionManager<T> {
    pub fn new_from_config(config: &Config) -> Result<Self> {
        Ok(match config.session.get(T::XDG_TYPE) {
            Some(session_config) => session_config.clone().try_into()?,
            None => Self {
                config: T::ManagerConfig::default(),
                entries: SessionMap::new(),
            },
        })
    }

    pub fn new_session(
        &self,
        name: &str,
        mut context: SessionContext,
        class: SessionClass,
    ) -> Result<process::Child> {
        let metadata = self.lookup_metadata(name)?;

        context.env = context
            .env
            .set(class)
            .set(SessionNameEnv(metadata.name.clone()))
            .set(SessionCompositionEnv::simple(metadata.name))
            .set(SessionTypeEnv(T::XDG_TYPE.to_string()));

        T::spawn_session(&self.config, metadata.executable, context)
    }

    pub fn lookup_metadata(&self, name: &str) -> Result<SessionMetadata> {
        if let Some(internal_entry) = self.entries.get(name) {
            return Ok(internal_entry.clone());
        };

        T::lookup_metadata(name)
    }

    pub fn lookup_metadata_all(&self) -> SessionMap {
        self.entries.clone().union(T::lookup_metadata_all())
    }
}

#[macro_export]
macro_rules! session_type {
    ($session_type:tt) => {
        crate::session::$session_type::Session
    };
}

#[macro_export]
macro_rules! sessions {
    ([$($session:tt),+]) => { // fn sessions([*session_types])
        $( pub mod $session; )+

        $crate::scope!{($all:tt) => {
            #[macro_export]
            macro_rules! _dispatch_session {
                ($xdg_type:expr => $function:ident($all($args:tt)*)) => { // string => function(*arguments)
                    match $xdg_type {
                        // for T in session_types:
                        //     T::XDG_TYPE => function::<T>(*arguments)
                        $( <session_type!($session)>::XDG_TYPE => $function::<session_type!($session)>($all($args)*), )+
                        //
                        other => anyhow::bail!("{other} is not a valid session type."),
                    }
                }
            }
            pub use _dispatch_session as dispatch_session; // return _dispatch_session
        }}
    }
}

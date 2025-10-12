use anyhow::{Context, Result};
use serde::{Deserialize, de::DeserializeOwned};

use std::{
    path::PathBuf,
    process::{self, Command},
};

use super::{context::SessionContext, metadata::SessionMetadataLookup};
use crate::{
    environment::{EnvContainer, EnvRecipient},
    session::metadata::{SessionMap, SessionMetadata},
    utils::config::Config,
};

pub mod prelude {
    pub use crate::session::{
        context::SessionContext,
        manager,
        metadata::{self, SessionMap, SessionMetadata},
    };
    pub use serde::Deserialize;
}

// TODO: is setting this to the SessionMetadata.name appropriate?
// The spec says this can contain list of values
crate::define_env!("XDG_CURRENT_DESKTOP", SessionNameEnv(String));

crate::define_env!("XDG_SESSION_TYPE", SessionTypeEnv(String));

pub trait SessionManager: Sized + Default + DeserializeOwned + SessionMetadataLookup {
    const XDG_TYPE: &str;
    type EnvDiff: EnvContainer;

    fn setup_session(&self, context: SessionContext) -> Result<Self::EnvDiff>;

    /// If you do not need fine-grained control over the startup flow
    /// consider implementing setup_session() instead
    fn spawn_session(
        &self,
        executable: PathBuf,
        context: SessionContext,
    ) -> Result<process::Child> {
        // Note: this is a cheap clone
        // As Env is immutable underneath
        let env = context.env.clone();

        let session_env = self.setup_session(context)?;

        Command::new(executable)
            .set_env(env.merge(session_env))
            .spawn()
            .context("Failed to spawn main session process")
    }
}

#[derive(Deserialize)]
pub struct GenericSessionManager<T> {
    #[serde(flatten)]
    manager: T,
    entries: SessionMap,
}

impl<M: SessionManager> GenericSessionManager<M> {
    pub fn new_from_config(config: &Config) -> Result<Self> {
        Ok(match config.session.get(M::XDG_TYPE) {
            Some(session_config) => session_config.clone().try_into()?,
            None => Self {
                manager: M::default(),
                entries: SessionMap::new(),
            },
        })
    }

    pub fn new_session(
        self,
        metadata: SessionMetadata,
        mut context: SessionContext,
    ) -> Result<process::Child> {
        context.env = context
            .env
            .set(SessionNameEnv(metadata.name))
            .set(SessionTypeEnv(M::XDG_TYPE.to_string()));

        self.manager.spawn_session(metadata.executable, context)
    }

    pub fn lookup_metadata(&self, name: &str) -> Result<SessionMetadata> {
        if let Some(internal_entry) = self.entries.get(name) {
            return Ok(internal_entry.clone());
        };

        M::lookup_metadata(name)
    }

    pub fn lookup_metadata_all(&self) -> SessionMap {
        self.entries.clone().union(M::lookup_metadata_all())
    }
}

#[macro_export]
macro_rules! session_type {
    ($session_type:tt) => {
        crate::session::$session_type::SessionManager
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

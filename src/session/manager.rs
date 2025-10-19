use anyhow::{Context, Result};
use serde::{Deserialize, de::DeserializeOwned};
use shrinkwraprs::Shrinkwrap;

use std::{any::Any, path::PathBuf, sync::mpsc};

use super::metadata::SessionMetadataLookup;
use crate::{
    environment::{EnvContainer, EnvContainerPartial, EnvRecipient, prelude::*},
    login::{VtRenderMode, context::LoginContext},
    session::metadata::{SessionMap, SessionMetadata},
    utils::config::Config,
};

pub mod prelude {
    pub use crate::{
        login::context::VtNumber,
        session::{
            manager::{self, SessionContext},
            metadata::{self, SessionMap, SessionMetadata},
        },
    };
    pub use serde::Deserialize;
}

pub trait SessionType: Sized + SessionMetadataLookup {
    const XDG_TYPE: &str;

    type ManagerConfig: Default + DeserializeOwned;
    type EnvDiff: EnvContainer;

    const VT_RENDER_MODE: VtRenderMode = VtRenderMode::Graphics;

    fn setup_session(
        config: &Self::ManagerConfig,
        context: &mut SessionContext,
    ) -> Result<Self::EnvDiff>;
}

type ExitReason = String;

#[derive(Shrinkwrap)]
pub struct SessionContext {
    #[shrinkwrap(main_field)]
    pub login_context: LoginContext,
    pub shutdown_tx: mpsc::Sender<ExitReason>,

    resources: Vec<Box<dyn Any>>,
}

impl SessionContext {
    pub fn persist(&mut self, resource: Box<dyn Any>) {
        self.resources.push(resource);
    }
}

pub struct SessionBuilder {
    context: SessionContext,
    shutdown_rx: mpsc::Receiver<ExitReason>,
}

impl SessionBuilder {
    fn new(login_context: LoginContext) -> Self {
        let (shutdown_tx, shutdown_rx) = mpsc::channel();

        Self {
            shutdown_rx,
            context: SessionContext {
                login_context,
                shutdown_tx,
                resources: Vec::new(),
            },
        }
    }

    fn finish(self) -> SessionInstance {
        SessionInstance {
            resources: self.context.resources,
            shutdown_rx: self.shutdown_rx,
        }
    }
}

pub struct SessionInstance {
    resources: Vec<Box<dyn Any>>,
    shutdown_rx: mpsc::Receiver<ExitReason>,
}

impl SessionInstance {
    pub fn join(self) -> Result<ExitReason> {
        let exit_reason = self
            .shutdown_rx
            .recv()
            .context("Tx end of session shutdown channel unexpectedly closed")?;

        drop(self.resources);
        Ok(exit_reason)
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

    pub fn spawn_session(
        &self,
        context: LoginContext,
        executable: PathBuf,
    ) -> Result<SessionInstance> {
        let mut builder = SessionBuilder::new(context);

        let session_env_diff = T::setup_session(&self.config, &mut builder.context)?;

        let mut command = builder.context.command(&executable);
        command.merge_env(session_env_diff.to_env()).unwrap();

        let _child = command
            .spawn()
            .context("Failed to spawn main session executable")?;

        // TODO: connect child.wait to shutdown_tx

        Ok(builder.finish())
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

define_env!("XDG_SESSION_TYPE", pub SessionTypeEnv(String));
env_parser_auto!(SessionTypeEnv);

impl<T: SessionType> EnvContainerPartial for SessionManager<T> {
    fn apply_as_container(&self, env: Env) -> Env {
        env.set(SessionTypeEnv(T::XDG_TYPE.to_string()))
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

use anyhow::{Context, Result};
use bon::{bon, builder};
use facet::Facet;
use shrinkwraprs::Shrinkwrap;
use tokio::{process::Command, sync::mpsc};

use std::{any::Any, collections::HashMap};

use crate::{
    environment::{EnvContainerPartial, prelude::*},
    login::context::LoginContext,
    session::{
        SessionInner, SessionType, SessionTypeTag,
        metadata::{SessionDefinition, SessionMetadataMap, Tagged},
    },
    utils::config::Config,
};

pub type ExitReason = String;

#[derive(Shrinkwrap)]
#[shrinkwrap(mutable)]
pub struct SessionContext {
    #[shrinkwrap(main_field)]
    pub login_context: LoginContext,
    pub shutdown_tx: mpsc::Sender<ExitReason>,

    resources: Vec<Box<dyn Any + Send>>,
}

impl SessionContext {
    pub fn persist(&mut self, resource: Box<dyn Any + Send>) {
        self.resources.push(resource);
    }

    pub fn spawn(&self, cmd: Command) -> Result<()> {
        self.login_context.spawn(cmd, self.shutdown_tx.clone())
    }
}

pub struct SessionInstance {
    resources: Vec<Box<dyn Any + Send>>,
    shutdown_rx: mpsc::Receiver<ExitReason>,
}

impl SessionInstance {
    pub async fn join(mut self) -> Result<ExitReason> {
        let exit_reason = self
            .shutdown_rx
            .recv()
            .await
            .context("Tx end of session shutdown channel unexpectedly closed")?;

        drop(self.resources);
        Ok(exit_reason)
    }
}

#[derive(Shrinkwrap)]
struct SessionData {
    #[shrinkwrap(main_field)]
    inner: SessionInner,
    config_entries: SessionMetadataMap,
}

impl SessionData {
    fn parse(tag: &SessionTypeTag, config: &Config) -> Result<Self> {
        // TODO: no clone
        let session_cfg = config.sessions.get(tag).cloned().unwrap_or_default();
        let inner = SessionInner::parse(tag, session_cfg.config)?;
        let config_entries = session_cfg.entries;

        Ok(Self {
            inner,
            config_entries,
        })
    }
}

pub struct SessionManager {
    data: HashMap<SessionTypeTag<String>, SessionData>,
}

#[bon]
impl SessionManager {
    #[builder]
    pub fn new(config: &Config, load_only: Option<&[&SessionTypeTag]>) -> Result<Self> {
        let mut data = HashMap::new();
        let load_types = load_only.unwrap_or(crate::session::ALL_TAGS);

        for session_type in load_types {
            data.insert(
                session_type.to_string(),
                SessionData::parse(&session_type, config)?,
            );
        }

        Ok(Self { data })
    }

    pub async fn run(
        &self,
        tag: Option<&SessionTypeTag>,
        login_context: LoginContext,
        definition: &SessionDefinition,
    ) -> Result<SessionInstance> {
        let session_manager = self.resolve_session(&definition.id, definition.)?;

        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

        let mut context = SessionContext {
            shutdown_tx,
            login_context,
            resources: Vec::new(),
        };

        session_manager
            .setup_session(&mut context, &definition.executable)
            .await?;

        Ok(SessionInstance {
            resources: context.resources,
            shutdown_rx,
        })
    }
}

define_env!("XDG_SESSION_TYPE", pub SessionTypeEnv(SessionTypeTag<String>));
env_parser_auto!(SessionTypeEnv);

impl EnvContainerPartial for SessionData {
    fn apply_as_container(&self, env: Env) -> Env {
        env.set(SessionTypeEnv(self.tag().to_string()))
    }
}

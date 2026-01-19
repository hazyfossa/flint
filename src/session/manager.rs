use anyhow::{Context, Result};
use shrinkwraprs::Shrinkwrap;
use tokio::{process::Command, sync::mpsc};

use std::any::Any;

use crate::{
    environment::{EnvContainerPartial, prelude::*},
    login::context::LoginContext,
    session::{Session, define::SessionTypeTag, metadata::SessionMetadata},
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

// TODO: consider managing env separately, which will simplify the manager to (T, executable: PathBuf)

pub struct SessionManager<T> {
    definition: SessionMetadata<T>,
    inner: T,
}

impl<T: Session> SessionManager<T> {
    pub async fn run(&self, login_context: LoginContext) -> Result<SessionInstance> {
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

        let mut context = SessionContext {
            shutdown_tx,
            login_context,
            resources: Vec::new(),
        };

        self.inner
            .setup_session(&mut context, &self.definition.executable)
            .await?;

        Ok(SessionInstance {
            resources: context.resources,
            shutdown_rx,
        })
    }
}

// TODO: unified env

define_env!("XDG_SESSION_TYPE", pub SessionTypeEnv(SessionTypeTag<String>));
env_parser_auto!(SessionTypeEnv);

impl<T: Session> EnvContainerPartial for SessionManager<T> {
    fn apply_as_container(&self, env: Env) -> Env {
        env.set(SessionTypeEnv(T::TAG.to_string()))
    }
}

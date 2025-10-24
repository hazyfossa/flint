use anyhow::{Context, Result};
use rustix::process::{self, Signal};
use serde::Deserialize;
use shrinkwraprs::Shrinkwrap;
use tokio::{process::Command, sync::mpsc};

use std::{any::Any, path::Path, process::ExitStatus};

use crate::{
    environment::{EnvContainer, EnvRecipient},
    login::context::LoginContext,
    session::{
        define::SessionType,
        metadata::{SessionDefinition, SessionMap},
    },
    utils::config::Config,
};

type ExitReason = String;

#[derive(Shrinkwrap)]
pub struct SessionContext {
    #[shrinkwrap(main_field)]
    pub login_context: LoginContext,
    pub shutdown_tx: mpsc::Sender<ExitReason>,

    resources: Vec<Box<dyn Any>>,
}

async fn handle_session_subprocess(cmd: Command, shutdown_tx: mpsc::Sender<ExitReason>) {
    // TODO: stdio handling

    let program_name = cmd.as_std().get_program().to_owned();

    async fn handler(mut cmd: Command) -> Result<ExitStatus> {
        let mut wait_token = cmd.spawn()?;

        let exit_code = wait_token.wait().await?;
        Ok(exit_code)
    }

    let ret = match handler(cmd).await {
        Ok(exit_status) => format!("{program_name:?} exited with {exit_status}"),
        Err(error) => format!("Error while handling {program_name:?}:\n{error}"),
    };

    // NOTE: we ignore error on send here, as if the shutdown channel is closed
    // we're most likely already shutting down
    let _ = shutdown_tx.send(ret).await;
}

impl SessionContext {
    pub fn persist(&mut self, resource: Box<dyn Any>) {
        self.resources.push(resource);
    }

    pub fn update_env<E: EnvContainer>(&mut self, variables: E) {
        self.login_context.env = self.login_context.env.clone().merge(variables)
    }

    pub fn spawn(&self, mut cmd: Command) -> Result<()> {
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

        let shutdown_tx = self.shutdown_tx.clone();
        tokio::spawn(handle_session_subprocess(cmd, shutdown_tx));

        Ok(())
    }
}

#[derive(Shrinkwrap)]
#[shrinkwrap(mutable)]
struct SessionBuilder {
    #[shrinkwrap(main_field)]
    context: SessionContext,
    shutdown_rx: mpsc::Receiver<ExitReason>,
}

impl SessionBuilder {
    fn new(login_context: LoginContext) -> Self {
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

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

#[derive(Deserialize)]
pub struct SessionManager<T: SessionType> {
    #[serde(flatten)]
    config: T::ManagerConfig,
    #[serde(rename = "entry")]
    entries: SessionMap,
}

impl<T: SessionType> SessionManager<T> {
    pub fn new_from_config(config: &Config) -> Result<Self> {
        Ok(match config.session.get(T::XDG_TYPE) {
            Some(session_config) => session_config
                .clone()
                .try_into()
                .context("Config error")
                .context(format!("Invalid config for \"[session.{}]\"", T::XDG_TYPE))?,

            None => Self {
                config: T::ManagerConfig::default(),
                entries: SessionMap::new(),
            },
        })
    }

    pub async fn spawn_session(
        &self,
        context: LoginContext,
        executable: &Path,
    ) -> Result<SessionInstance> {
        let mut builder = SessionBuilder::new(context);

        T::setup_session(&self.config, &mut builder, executable).await?;

        builder
            .terminal
            .activate(T::VT_RENDER_MODE)
            .context("Failed to swtich to session VT")?;

        Ok(builder.finish())
    }

    pub fn lookup_metadata(&self, name: &str) -> Result<SessionDefinition> {
        if let Some(internal_entry) = self.entries.get(name) {
            return Ok(internal_entry);
        };

        T::lookup_metadata(&name)
    }

    pub fn lookup_metadata_all(&self) -> SessionMap {
        self.entries.clone().union(T::lookup_metadata_all())
    }
}

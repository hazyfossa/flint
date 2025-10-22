use anyhow::{Context, Result};
use rustix::process::{self, Signal};
use serde::Deserialize;
use shrinkwraprs::Shrinkwrap;

use std::{
    any::Any, os::unix::process::CommandExt, path::PathBuf, process::Command, sync::mpsc, thread,
};

use crate::{
    environment::{EnvContainer, EnvRecipient},
    login::context::LoginContext,
    session::{
        define::SessionType,
        metadata::{SessionMap, SessionMetadata},
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

impl SessionContext {
    pub fn persist(&mut self, resource: Box<dyn Any>) {
        self.resources.push(resource);
    }

    pub fn update_env<E: EnvContainer>(&mut self, variables: E) {
        self.login_context.env = self.login_context.env.clone().merge(variables)
    }

    pub fn spawn(&self, mut cmd: Command) -> Result<()> {
        // TODO: custom stream processing on stdio

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
        let mut wait_token = cmd.spawn()?;

        let shutdown_tx = self.shutdown_tx.clone();

        // TODO: optimize
        thread::spawn(move || {
            let exit_status = wait_token.wait().unwrap();
            shutdown_tx.send(format!("{:?} exited with {exit_status}", cmd.get_program()))
        });

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

        T::setup_session(&self.config, &mut builder, executable)?;

        builder
            .terminal
            .activate(T::VT_RENDER_MODE)
            .context("Failed to swtich to session VT")?;

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

use std::process::ExitStatus;

use anyhow::Result;
use rustix::process::{self, Signal};
use tokio::{process::Command, sync::mpsc};

use crate::{environment::EnvRecipient, session::manager::ExitReason};

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

impl super::context::LoginContext {
    pub fn spawn(&self, mut cmd: Command, shutdown_tx: mpsc::Sender<ExitReason>) -> Result<()> {
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
        tokio::spawn(handle_session_subprocess(cmd, shutdown_tx));

        Ok(())
    }
}

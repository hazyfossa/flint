#![allow(dead_code)]

mod bind;
mod environment;
mod utils;
mod worker;

use std::{os::fd::AsFd, path::PathBuf};

use anyhow::{Context, Result};
use argh::FromArgs;
use envy::Get;
use serde::{Deserialize, Serialize};

use crate::{
    bind::{
        pam::{CredentialsOP, Pam, PamDisplay},
        tty::{Terminal, VtNumber},
    },
    environment::Seat,
};

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    #[allow(dead_code)]
    version: Option<String>,
}

struct View {
    vt: Option<VtNumber>,
    seat: Option<Seat>,
}

struct PamSession {
    pam: Pam,
}

impl PamSession {
    fn start(
        env: impl envy::Diff,
        username: Option<&str>,
        display: impl PamDisplay,
        require_auth: bool,
        silent: bool,
    ) -> Result<Self> {
        let mut pam = Pam::new("flint", display, username, silent)?;

        if require_auth {
            pam.authenticate(false)?;
        }
        pam.assert_account_is_valid(false)?;
        pam.credentials(CredentialsOP::Establish)?;

        pam.set_env(env)?;
        pam.open_session()?;

        Ok(Self { pam })
    }

    fn view(&self) -> View {
        View {
            vt: self.pam.get::<VtNumber>().ok(),
            seat: self.pam.get::<Seat>().ok(),
        }
    }
}

impl Drop for PamSession {
    fn drop(&mut self) {
        self.pam.close_session().unwrap();
        self.pam.credentials(CredentialsOP::Delete).unwrap();
    }
}

fn new_process_tree_session<F: AsFd>(ctty: &Terminal<F>) -> Result<()> {
    rustix::process::setsid().context("Failed to create a new process-tree session (setsid)")?;

    ctty.set_as_ctty()
        .context("Failed to set controlling tty")?;

    Ok(())
}

#[derive(FromArgs)]
/// flint session manager
struct Args {
    /// configuration path
    #[argh(option)]
    #[argh(default = r#""/etc/flint.toml".into()"#)]
    config: PathBuf,

    /// TODO
    #[argh(option)]
    #[argh(default = "false")]
    can_suspend_home: bool,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber)?;

    let args: Args = argh::from_env();

    Ok(())
}

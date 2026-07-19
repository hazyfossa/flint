#![allow(dead_code)]

mod environment;
mod plymouth;
mod seat;
mod systemd;
mod tty;
mod utils;
mod xdg_session;

use std::{os::fd::AsFd, path::PathBuf};

use anyhow::{Context, Result};
use argh::FromArgs;

use flint_pam::{CredentialsOP, Pam, PamDisplay};
use hazymacros::newtype;
use log::warn;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::{
    seat::SeatID,
    tty::{Terminal, VtNumber},
    utils::warn::WarnExt,
};

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    #[allow(dead_code)]
    version: Option<String>,
}

newtype!(SessionID = u64);

enum View {
    Vt(VtNumber),
    Seat(SeatID),
}

impl View {
    fn seat(&self) -> Option<SeatID> {
        match self {
            Self::Seat(x) => Some(x.clone()),
            _ => None,
        }
    }

    fn from_env(env: &impl envy::Get) -> Option<Self> {
        let seat = env
            .maybe_get::<SeatID>()
            .context("Proceeding without seat")
            .warn()
            .flatten();

        let seat = seat.filter(|x| !x.is_seat0());

        let vt = env
            .maybe_get::<VtNumber>()
            .context("invalid vt")
            .warn()
            .flatten();

        if let Some(x) = seat {
            if vt.is_some() {
                warn!("Both non-zero seat and vt specified, ignoring vt");
            }

            Some(Self::Seat(x))
        } else {
            vt.map(Self::Vt)
        }
    }
}

struct PamSession {
    pam: Pam,
}

impl PamSession {
    fn start(
        env: impl envy::Diff,
        username: Option<&str>,
        display: Option<impl PamDisplay>,
        require_auth: bool,
    ) -> Result<Self> {
        let mut pam = Pam::new("flint", display, username)?;

        if require_auth {
            pam.authenticate(false)?;
        }
        pam.assert_account_is_valid(false)?;
        pam.credentials(CredentialsOP::Establish)?;

        pam.set_env(env)?;
        pam.open_session()?;

        Ok(Self { pam })
    }

    fn view(&self) -> Result<View> {
        // TODO: this should never be necessary under new model
        View::from_env(&self.pam)
            .context("Could not get seat/vt from PAM env. Check if systemd_pam is in the stack.")
    }
}

impl Drop for PamSession {
    fn drop(&mut self) {
        self.pam.close_session().unwrap();
        self.pam.credentials(CredentialsOP::Delete).unwrap();
    }
}

// TODO: does pam do this for us?
fn new_shell_session<F: AsFd>(ctty: &Terminal<F>) -> Result<()> {
    rustix::process::setsid().context("Failed to create a new process-tree session (setsid)")?;

    ctty.set_as_ctty()
        .context("Failed to set controlling tty")?;

    Ok(())
}

// Session handle is a session placed on a seat
struct SessionHandle {
    id: SessionID,
    shutdown: broadcast::Receiver<()>,
}

#[derive(FromArgs)]
/// flint session manager
struct Args {
    /// configuration path
    #[argh(option)]
    #[argh(default = r#""/etc/flint.toml".into()"#)]
    config: PathBuf,

    /// TODO
    #[argh(switch)]
    can_suspend_home: bool,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().init();
    let args: Args = argh::from_env();

    Ok(())
}

#![allow(dead_code)]

mod greet;
mod metadata;
mod plymouth;
mod seat;
mod session;
mod tty;
mod user;
mod utils;

use std::{collections::HashMap, os::fd::AsFd, path::PathBuf};

use anyhow::{Context, Result};
use argh::FromArgs;

use flint_pam::{CredentialsOP, Pam, PamDisplay};
use hazymacros::newtype;
use log::warn;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::{
    seat::{SeatEvent, SeatID, SeatManagerObject},
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

// broadcast is wrong here, session-triggered shutdown will propagate to seat,
// seat to global. Need hierarchical channels.
type ShutdownRx = broadcast::Receiver<()>;
type ShutdownTx = broadcast::Sender<()>;

struct SeatHandle {
    id: SeatID,
    shutdown: ShutdownRx,
}

// Session handle is a session placed on a seat
struct SessionHandle {
    id: SessionID,
    view: View,
    shutdown: ShutdownRx,
}

struct Flint {
    seat_manager: SeatManagerObject,
    seats: HashMap<SeatID, ShutdownRx>,
}

impl Flint {
    async fn new() -> Result<Self> {
        let seat_manager = seat::seat_manager()
            .await
            .context("Failed to connect to seat manager")?;

        Ok(Self {
            seat_manager,
            seats: HashMap::new(),
        })
    }

    // fn get_seat(&mut self, id: SeatID) -> Result<SeatHandle> {
    //     match self.seats.entry(id.clone()) {
    //         hash_map::Entry::Occupied(x) => Ok(SeatHandle(x)),
    //         hash_map::Entry::Vacant(_) => bail!("seat {} not found or not managed by flint", *id),
    //     }
    // }

    fn new_seat(&mut self, id: SeatID) {}

    async fn run(mut self) {
        for id in self.seat_manager.list_seats().await {
            self.new_seat(id);
        }

        while let Some((id, event)) = self.seat_manager.next_event().await {
            match event {
                SeatEvent::Added => self.new_seat(id),
                SeatEvent::Changed => todo!(),
                SeatEvent::Removed => todo!(),
            }
        }
    }
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

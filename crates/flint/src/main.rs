#![allow(dead_code)]

mod core;
mod driver;
mod greet;
mod metadata;
mod seat;
mod user;
mod utils;

// TODO: bring back (not a priority)
// mod plymouth;

use std::{collections::HashMap, path::PathBuf};

use anyhow::{Context, Result};
use argh::FromArgs;

use hazymacros::newtype;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::seat::{SeatEvent, SeatID, SeatManagerObject};

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    #[allow(dead_code)]
    version: Option<String>,
}

newtype!(SessionID = u64);

// broadcast is wrong here, session-triggered shutdown will propagate to seat,
// seat to global. Need hierarchical channels.
type ShutdownRx = broadcast::Receiver<()>;
type ShutdownTx = broadcast::Sender<()>;

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

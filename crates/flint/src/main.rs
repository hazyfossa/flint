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

use std::{
    collections::HashMap,
    io::{BufReader, ErrorKind},
    path::PathBuf,
    sync::OnceLock,
};

use anyhow::{Context, Result};
use argh::FromArgs;

use fs_err::File;
use hazymacros::newtype;
use serde::{Deserialize, Serialize};
use static_reload::{Resource, ResourceCell};
use tokio::sync::broadcast;

use crate::{
    seat::{SeatEvent, SeatID, SeatManagerObject, view::View},
    utils::warn::WarnExt,
};

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    #[allow(dead_code)]
    version: Option<String>,
}

pub static CONFIG: ResourceCell<Config> = ResourceCell::new();

impl Resource for Config {
    type Definition = PathBuf;
    type Error = anyhow::Error;

    async fn load(definition: &Self::Definition) -> Result<Self> {
        let file = match File::open(definition) {
            Err(e) if matches!(e.kind(), ErrorKind::NotFound) => return Ok(Config::default()),
            other => other,
        }?;

        let buf = BufReader::new(file);

        Ok(serde_json::from_reader(buf)?)
    }
}

newtype!(SessionID = u64);

// broadcast is wrong here, session-triggered shutdown will propagate to seat,
// seat to global. Need hierarchical channels.
type ShutdownRx = broadcast::Receiver<()>;
type ShutdownTx = broadcast::Sender<()>;

struct Flint {
    seat_manager: SeatManagerObject,
    seats: HashMap<SeatID, View>,
}

impl Flint {
    // fn get_seat(&mut self, id: SeatID) -> Result<SeatHandle> {
    //     match self.seats.entry(id.clone()) {
    //         hash_map::Entry::Occupied(x) => Ok(SeatHandle(x)),
    //         hash_map::Entry::Vacant(_) => bail!("seat {} not found or not managed by flint", *id),
    //     }
    // }

    async fn new_seat(&mut self, id: SeatID) {
        let props = self
            .seat_manager
            .query(&id)
            .await
            .context("Failed to get properties, skipping seat")
            .warn();

        if let Some(props) = props {
            self.seats.insert(id, props.view);
            // TODO: spawn greeter here
        }
    }

    async fn run(mut self) {
        // TODO: parallelize
        for id in self.seat_manager.list_seats().await {
            self.new_seat(id).await;
        }

        while let Some((id, event)) = self.seat_manager.next_event().await {
            match event {
                SeatEvent::Added => self.new_seat(id).await,
                SeatEvent::Changed => todo!(), // TODO: here, view can change if vt
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
    CONFIG.init(args.config).await?;

    Ok(())
}

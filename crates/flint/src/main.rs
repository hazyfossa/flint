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
    io::{BufReader, ErrorKind},
    path::PathBuf,
};

use argh::FromArgs;
use eyre::Result;
use fs_err::File;
use hazymacros::newtype;
use serde::{Deserialize, Serialize};
use static_resource::{Resource, ResourceCell};
use tokio::sync::broadcast;

use crate::{core::Runtime, seat::SeatManager};

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    #[allow(dead_code)]
    version: Option<String>,
    default_runtime: Runtime,
}

pub static CONFIG: ResourceCell<Config> = ResourceCell::new();

impl Resource for Config {
    type Definition = PathBuf;
    type Error = eyre::Error;

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
    seat_manager: SeatManager::Object,
}

impl Flint {}

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

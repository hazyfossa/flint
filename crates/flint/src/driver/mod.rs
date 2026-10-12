use std::path::PathBuf;

use bon::Builder;
use envy::define_env;
use eyre::Result;
use plug::plug;

use crate::metadata;

// https://www.freedesktop.org/software/systemd/man/latest/pam_systemd.html#type=
define_env!(SessionTypeEnv(String) = "XDG_SESSION_TYPE");

pub enum Kind {
    Graphical,
    Text,
}

#[derive(Builder)]
pub struct DriverMeta {
    /// Whether this driver's tag is registered as an XDG session type
    #[builder(default = false)]
    xdg_registered: bool,

    xdg_lookup_path: Option<PathBuf>,

    #[builder(default = Kind::Graphical)]
    view_kind: Kind,

    #[builder(default = Vec::new())]
    intrinsic_sessions: Vec<(metadata::IntrinsicTag, metadata::Summary)>,
}

// #[plug]
pub trait Driver {
    fn meta() -> DriverMeta;

    async fn run(&self, cx: u128 /* TODO */) -> Result<impl envy::Diff>;

    /// May provide a hint that this driver is unavailable at runtime.
    /// Only a hint. Drivers should not rely on this for internal safety.
    async fn available() -> bool {
        true
    }
}

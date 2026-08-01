use std::path::PathBuf;

use anyhow::Result;
use bon::Builder;
use envy::define_env;

// https://www.freedesktop.org/software/systemd/man/latest/pam_systemd.html#type=
define_env!(SessionTypeEnv(String) = "XDG_SESSION_TYPE");

pub enum Kind {
    Graphical,
    Text,
}

#[derive(Builder)]
pub struct DriverMeta {
    xdg_registered: bool,
    xdg_lookup_path: Option<PathBuf>,

    #[builder(default = Kind::Graphical)]
    view_kind: Kind,
}

pub trait Driver {
    fn meta(&self) -> DriverMeta;

    async fn run(&self, cx: u128 /* TODO */) -> Result<impl envy::Diff>;

    // TODO: only instrinsic sessions require this (to not display) and these are postponed.
    // async fn supported(&self) -> bool {
    //     true
    // }
}

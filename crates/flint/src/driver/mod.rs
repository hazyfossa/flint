use anyhow::Result;
use bon::Builder;
use envy::define_env;

// https://www.freedesktop.org/software/systemd/man/latest/pam_systemd.html#type=
define_env!(SessionTypeEnv(String) = "XDG_SESSION_TYPE");

#[derive(Builder)]
pub struct DriverMeta {}

pub trait Driver {
    // fn xdg_lookup_path() -> &'static str {
    //     ""
    // }

    // fn special_sessions() -> Vec<SessionMeta> {
    //     Vec::new()
    // }

    async fn run(&self, cx: u128 /* TODO */) -> Result<impl envy::Diff>;
}

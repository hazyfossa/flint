use serde::Deserialize;

use crate::{
    define_env,
    environment::{Env, EnvContainer},
    template::{self, FreedesktopMetadata},
};

define_env!("WAYLAND_DISPLAY", Display(String));

pub struct Session {
    display: Display,
}

impl FreedesktopMetadata for Session {
    const LOOKUP_PATH: &str = "/usr/share/wayland-sessions/";
}

impl template::Session for Session {
    const XDG_TYPE: &str = "wayland";
    type Manager = Manager;
}

impl EnvContainer for Session {
    fn env_diff(self) -> crate::environment::Env {
        Env::new().set(self.display)
    }
}

#[derive(Deserialize)]
pub struct Manager;

impl Default for Manager {
    fn default() -> Self {
        Self {}
    }
}

impl template::SessionManager<Session> for Manager {
    fn setup_session(self) -> anyhow::Result<Session> {
        todo!()
    }
}

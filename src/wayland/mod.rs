use crate::{define_env, environment::EnvDiff, template};

define_env!("WAYLAND_DISPLAY", Display(String));

pub struct Session {
    display: Display,
}

impl template::Session for Session {
    const XDG_TYPE: &str = "wayland";
    const LOOKUP_PATH: &str = "/usr/share/wayland-sessions/";

    type Manager = Manager;

    fn env(self) -> crate::environment::EnvDiff {
        EnvDiff::build().set(self.display).build()
    }
}

pub struct Manager;

impl template::SessionManager<Session> for Manager {
    fn setup_session(self) -> anyhow::Result<Session> {
        todo!()
    }
}

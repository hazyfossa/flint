use crate::{define_env, environment::EnvDiff, template};

define_env!("WAYLAND_DISPLAY", Display(String));

pub struct WaylandSession {
    display: Display,
}

impl template::Session for WaylandSession {
    const XDG_TYPE: &str = "wayland";
    const LOOKUP_PATH: &str = "/usr/share/wayland-sessions/";

    type Manager = WaylandManager;

    fn env(self) -> crate::environment::EnvDiff {
        EnvDiff::build().set(self.display).build()
    }
}

pub struct WaylandManager;

impl template::SessionManager<WaylandSession> for WaylandManager {
    fn setup_session(self) -> anyhow::Result<WaylandSession> {
        todo!()
    }
}

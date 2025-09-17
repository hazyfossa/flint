use crate::{define_env, template};

define_env!("WAYLAND_DISPLAY", Display(String));

pub struct WaylandSession;

impl template::Session for WaylandSession {
    const XDG_TYPE: &str = "wayland";
    const LOOKUP_PATH: &str = "/usr/share/wayland-sessions/";

    type Manager = WaylandManager;

    fn env(self) -> crate::environment::EnvDiff {
        todo!()
    }
}

pub struct WaylandManager;

impl template::SessionManager<WaylandSession> for WaylandManager {
    fn setup_session(self) -> anyhow::Result<WaylandSession> {
        todo!()
    }
}

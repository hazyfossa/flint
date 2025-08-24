use crate::template;

struct Session;

impl template::Session for Session {
    const XDG_TYPE: &str = "wayland";
    const LOOKUP_PATH: &str = "/usr/share/wayland-sessions/";

    fn env(self) -> crate::environment::EnvDiff {
        todo!()
    }
}

use crate::session::SessionType;

struct Session;

impl SessionType for Session {
    const XDG_TYPE: &str = "wayland";
    const LOOKUP_PATH: &str = "/usr/share/wayland-sessions/";
}

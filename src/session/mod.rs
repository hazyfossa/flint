pub mod define;
pub mod manager;
pub mod metadata;

define::sessions!([x11, wayland, tty]);
pub use define::SessionType as Session;

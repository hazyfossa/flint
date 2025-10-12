pub mod context;
pub mod manager;
pub mod metadata;

crate::sessions!([x11, wayland, tty]);

pub mod context;
pub mod manager;

crate::sessions!([x11, wayland, tty]);

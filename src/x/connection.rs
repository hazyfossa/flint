use super::Display;

use anyhow::{Context, Result};
use libxauth::Cookie;

pub use x11rb::rust_connection::RustConnection as Connection;
use x11rb::{
    reexports::x11rb_protocol::parse_display::ConnectAddress, rust_connection::DefaultStream,
};

pub fn connect(display: Display, cookie: &Cookie) -> Result<Connection> {
    // simply doing xclient::connect will do a lot of unnecessary heuristics
    // we should have enough information to setup a connection manually

    let (auth_name, auth_data) = cookie.raw_data();

    // The PeerPath is irrelevant to us
    // change ::connect to a direct constructor, when one is implemented for abstract streams
    let (stream, _) = DefaultStream::connect(&ConnectAddress::Socket(display.local_socket()))
        .context("Failed connecting to local socket")?;

    Connection::connect_to_stream_with_auth_info(
        stream,
        0, // TODO: screen handling will require proper display parsing
        auth_name.into(),
        auth_data,
    )
    .context("Failed to connect to X11. Possibly auth rejected")
}

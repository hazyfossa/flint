use anyhow::{Context, Result};

use crate::{systemd, utils::config::Config};

pub async fn run(config: Config) -> Result<()> {
    let dbus = zbus::Connection::system()
        .await
        .context("Cannot conntext to DBus")?;

    let logind = systemd::dbus::logind::LoginD::builder(&dbus)
        .cache_properties(zbus::proxy::CacheProperties::No)
        .build()
        .await
        .context("Cannot connect to LoginD on dbus")?;

    Ok(())
}

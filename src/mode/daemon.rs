use anyhow::{Context, Result, anyhow};
use serde::Deserialize;

use crate::{systemd, utils::config::Config};

#[derive(Debug, Deserialize)]
pub struct DaemonConfig {
    greeter: String,
}

pub async fn run(config: Config) -> Result<()> {
    // TODO: this is extremely user-unfriendly
    // consider the case where you have set up to run daemon instead of tty1
    // and it fails with this
    // yet you cannot set up a config: you're logged out!
    // TODO: default greeter?
    let daemon_config = config
        .daemon
        .ok_or(anyhow!("You need to set up daemon.greeter in config."))?;

    let greeter_config = config.greeters.get(&daemon_config.greeter).ok_or(anyhow!(
        "daemon.greeter is invalid: {} is not found",
        daemon_config.greeter
    ));

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

mod bind;
mod environment;
mod utils;
mod worker;

use std::path::PathBuf;

use anyhow::{Context, Result};
use argh::FromArgs;
use rustix::process;
use serde::{Deserialize, Serialize};

use crate::{
    bind::{
        pam::{CredentialsOP, PAM, PamDisplay, PamItemType},
        tty::{Terminal, VtRenderMode},
    },
    environment::{Seat, VtNumber},
};

#[derive(FromArgs)]
/// flint session manager
struct Args {
    /// configuration path
    #[argh(option)]
    #[argh(default = r#""/etc/flint.toml".into()"#)]
    config: PathBuf,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    #[allow(dead_code)]
    version: Option<String>,
}

struct View {
    vt: VtNumber,
    seat: Option<Seat>,
}

fn start_session(
    display: impl PamDisplay,
    env: impl envy::diff::Diff,

    // If unset, it will be queried via PAM
    username: Option<&str>,

    executable: PathBuf,

    require_auth: bool,
    silent: bool,
) -> Result<()> {
    let mut pam = PAM::new("flint", display, username, silent)?;

    if require_auth {
        pam.authenticate(false)?;
    }
    pam.assert_account_is_valid(false)?;
    pam.credentials(CredentialsOP::Establish)?;

    process::setsid().context("failed to become a session leader process")?;

    let vt_number = match vt_number {
        Some(value) => value,
        None => todo!(), // TODO: vt alloc
    };

    let terminal = Terminal::new(vt_number).context("failed to provision an active VT")?;
    terminal
        .set_as_current()
        .context("failed to set terminal as current")?;

    pam.set_item(PamItemType::TTY, &vt_number.to_tty_string())?;

    pam.open_session()?;

    let pam_env = pam.get_env()?;
    let mut context = SessionContext::from_trusted_env(executable, pam_env)
        .context("failed to provision session context from env that PAM passed us")?;

    let session_resources = ();

    terminal
        .activate(VtRenderMode::Graphics)
        .context("failed to activate VT")?;

    // TODO: wait on end of graphical-session target in systemd.

    pam.close_session()?;
    drop(session_resources);
    pam.credentials(CredentialsOP::Delete)?;
    pam.end()?;

    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber)?;

    let mut args = Args::from_env();
    Ok(())
}

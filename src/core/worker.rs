use std::path::PathBuf;

use anyhow::{Context, Result};
use rustix::process;

use crate::{
    APP_NAME,
    bind::{
        pam::{CredentialsOP, PAM, PamDisplay, PamItemType},
        tty::{Terminal, VtNumber, VtRenderMode},
    },
    core::{View, environment::Seat},
    frame::environment::Env,
    session::SessionTypePlug,
};

pub struct SessionContext {
    pub view: View,
    pub env: Env,
    pub executable: PathBuf,

    uid: u32,
    gid: u32,

    systemd_user: u128, // TODO
}

async fn start_session<T: SessionTypePlug>(
    display: impl PamDisplay,

    // If unset, it will be queried via PAM
    username: Option<&str>,

    session_manager: T,
    executable: PathBuf,

    seat: Seat,
    // If unset, next available one will be allocated
    vt_number: Option<VtNumber>,

    require_auth: bool,
    silent: bool,
) -> Result<()> {
    let mut pam = PAM::new(APP_NAME, display, username, silent)?;

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

    let session_resources = session_manager.setup_session(&mut context).await?;

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

pub mod context;
mod pam;
mod tty;
pub mod users;

pub use tty::control::RenderMode as VtRenderMode;

use anyhow::{Context, Result};
use rustix::process;

use crate::{
    APP_NAME,
    environment::{Env, EnvRecipient},
    login::{
        context::{LoginContext, Seat, SessionClass},
        pam::{CredentialsOP, PamDisplay, PamItemType},
        tty::{ActiveVT, VtNumber},
        users::UserInfoProvider,
    },
    session::{define::SessionType, manager::SessionManager, metadata::SessionMetadata},
};

#[allow(dead_code)]
// NOTE: while technically PAM can query for a username
// for now we work around that
async fn login_worker<T: SessionType>(
    display: impl PamDisplay,

    user_info_provider: impl UserInfoProvider,
    // If unset, it will be queried via PAM
    username: Option<&str>,

    inherit_env: Env,
    seat: Seat,
    vt_number: VtNumber,

    session_manager: SessionManager<T>,
    session_class: SessionClass,
    session_metadata: SessionMetadata,

    require_auth: bool,
    silent: bool,
) -> Result<String> {
    let mut pam = pam::PAM::new(APP_NAME, display, username, silent)?;

    if require_auth {
        pam.authenticate(false)?;
    }
    pam.assert_account_is_valid(false)?;
    pam.credentials(CredentialsOP::Establish)?;

    let user_info = user_info_provider.query(&pam.get_username()?)?;
    let user_id = user_info.as_user_id();

    process::setsid().context("Failed to become a session leader process")?;

    let env = inherit_env
        .set(session_class)
        .merge(user_info)
        .merge_from(&session_manager)
        .merge_from(&session_metadata);

    pam.set_env(env)?;
    pam.open_session()?;
    let env = pam.get_env()?;

    let terminal = ActiveVT::new(vt_number).context("Failed to provision an active VT")?;
    terminal
        .set_as_current()
        .context("failed to set terminal as current")?;

    pam.set_item(PamItemType::TTY, &vt_number.to_tty_string())?;
    let env = env.set(vt_number);

    let context = LoginContext::new(env, seat, terminal, user_id)
        .context("Cannot establish a login context")?;

    let session = session_manager.spawn_session(context, session_metadata.executable)?;

    let exit_reason = session.join().await?;

    pam.close_session()?;
    pam.credentials(CredentialsOP::Delete)?;
    pam.end()?;

    Ok(exit_reason)
}

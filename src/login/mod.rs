#![allow(dead_code)]
pub mod context;
mod pam;
mod tty;
pub mod users;

use context::{LoginContext, SessionClass};
pub use tty::control::RenderMode as VtRenderMode;

use anyhow::{Context, Result};
use rustix::process::setsid;

use crate::{
    APP_NAME,
    environment::{Env, EnvRecipient},
    login::users::UserInfoProvider,
    session::{
        manager::{SessionManager, SessionType},
        metadata::SessionMetadata,
    },
};
use pam::{CredentialsOP, PamDisplay};

// NOTE: while technically PAM can query for a username
// for now we work around that
fn login<T: SessionType>(
    display: impl PamDisplay,

    user_info_provider: impl UserInfoProvider,
    // If unset, it will be queried via PAM
    username: Option<&str>,

    inherit_env: Env,

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
    let user_switch = user_info.as_user_id();

    setsid().context("Failed to become a session leader process")?;

    let env = inherit_env
        .set(session_class)
        .merge(user_info)
        .merge_from(&session_manager)
        .merge_from(&session_metadata);

    pam.set_env(env)?;
    pam.open_session()?;
    let env = pam.get_env()?;

    let context = LoginContext::from_env(env, Some(user_switch))?;
    let session = session_manager.spawn_session(context, session_metadata.executable)?;

    let exit_reason = session.join()?;

    pam.close_session()?;
    pam.credentials(CredentialsOP::Delete)?;
    pam.end()?;

    Ok(exit_reason)
}

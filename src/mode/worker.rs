use anyhow::{Context, Result};
use pico_args::Arguments;
use rustix::process;
use tokio::net::UnixDatagram;

use crate::{
    APP_NAME,
    environment::{Env, EnvRecipient},
    login::{
        context::{LoginContext, Seat, SessionClass},
        pam::{CredentialsOP, PAM, PamDisplay, PamItemType},
        tty::{Terminal, VtNumber},
        users::UserInfoProvider,
    },
    session::{define::SessionType, manager::SessionManager, metadata::SessionDefinition},
};

#[allow(dead_code)]
// NOTE: while technically PAM can query for a username
// for now we work around that
async fn login<T: SessionType>(
    display: impl PamDisplay,

    user_info_provider: impl UserInfoProvider,
    // If unset, it will be queried via PAM
    username: Option<&str>,

    inherit_env: Env,
    seat: Seat,
    vt_number: VtNumber,

    session_manager: SessionManager<T>,
    session_class: SessionClass,
    session_metadata: SessionDefinition,

    require_auth: bool,
    silent: bool,
) -> Result<String> {
    let mut pam = PAM::new(APP_NAME, display, username, silent)?;

    if require_auth {
        pam.authenticate(false)?;
    }
    pam.assert_account_is_valid(false)?;
    pam.credentials(CredentialsOP::Establish)?;

    let user_info = user_info_provider.query(&pam.get_username()?)?;
    let user_id = user_info.as_user_id();

    process::setsid().context("Failed to become a session leader process")?;

    let executable = session_metadata.executable.clone();
    let env = inherit_env
        .set(session_class)
        .merge(user_info)
        .merge(session_metadata)
        .merge_from(&session_manager);

    pam.set_env(env)?;
    pam.open_session()?;
    let env = pam.get_env()?;

    let terminal = Terminal::new(vt_number).context("Failed to provision an active VT")?;
    terminal
        .set_as_current()
        .context("failed to set terminal as current")?;

    pam.set_item(PamItemType::TTY, &vt_number.to_tty_string())?;
    let env = env.set(vt_number);

    let context = LoginContext::new(env, seat, Some(terminal), user_id)
        .context("Cannot establish a login context")?;

    let session = session_manager.spawn_session(context, &executable).await?;

    let exit_reason = session.join().await?;

    pam.close_session()?;
    pam.credentials(CredentialsOP::Delete)?;
    pam.end()?;

    Ok(exit_reason)
}

async fn run(args: &Arguments) -> Result<()> {
    todo!()
}

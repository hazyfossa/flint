use anyhow::{Context, Result};
use pico_args::Arguments;
use rustix::process;

use crate::{
    APP_NAME,
    environment::{Env, EnvRecipient},
    login::{
        context::{LoginContext, Seat, SessionClass},
        pam::{CredentialsOP, PAM, PamDisplay, PamItemType},
        tty::{Terminal, VtNumber},
        users::UserInfoProvider,
    },
    session::{SessionType, SessionTypeData, metadata::SessionDefinition},
};

#[allow(dead_code)]
// NOTE: while technically PAM can query for a username
// for now we work around that
async fn login(
    display: impl PamDisplay,

    user_info_provider: impl UserInfoProvider,
    // If unset, it will be queried via PAM
    username: Option<&str>,

    inherit_env: Env,
    seat: Seat,
    vt_number: VtNumber,

    session_manager: SessionTypeData,
    session_class: SessionClass,
    session_definition: SessionDefinition,

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

    let env = inherit_env
        .set(session_class)
        .merge(user_info)
        .merge_from(&session_definition)
        .merge_from(&session_manager);

    let vt_mode = session_manager.vt_render_mode();
    let terminal = Terminal::new(vt_number).context("Failed to provision an active VT")?;
    terminal
        .set_as_current()
        .context("failed to set terminal as current")?;

    pam.set_item(PamItemType::TTY, &vt_number.to_tty_string())?;
    let env = env.set(vt_number);

    pam.set_env(env)?;
    pam.open_session()?;
    let env = pam.get_env()?;

    let context = LoginContext::new(env, seat, Some(vt_number), user_id)
        .context("Cannot establish a login context")?;

    let session = session_manager.run(context, &session_definition).await?;

    terminal
        .activate(vt_mode)
        .context("failed to activate VT")?;

    let exit_reason = session.join().await?;

    pam.close_session()?;
    pam.credentials(CredentialsOP::Delete)?;
    pam.end()?;

    Ok(exit_reason)
}

async fn run(args: &Arguments) -> Result<()> {
    todo!()
}

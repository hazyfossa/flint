#![allow(dead_code)]
mod pam;

use anyhow::{Context, Result};
use rustix::process::setsid;

use crate::{APP_NAME, environment::Env};
use pam::{CredentialsOP, PamDisplay};

// NOTE: this is a stub
struct UserInfo {
    username: String,
    uid: u32,
    gid: u32,
}

trait UserInfoProvider {
    fn query(&self, name: &str) -> Result<UserInfo>;
}

// NOTE: while technically PAM can query for a username
// for now we work around that
fn login(
    display: impl PamDisplay,

    user_info_provider: impl UserInfoProvider,
    default_username: Option<&str>,

    inherit_env: Env,

    require_auth: bool,
    silent: bool,
) -> Result<()> {
    let mut pam = pam::PAM::new(APP_NAME, display, default_username, silent)?;

    if require_auth {
        pam.authenticate(false)?;
    }
    pam.assert_account_is_valid(false)?;
    pam.credentials(CredentialsOP::Establish)?;

    let user = user_info_provider.query(&pam.get_username()?);

    setsid().context("Failed to become a session leader process")?;

    //

    pam.close_session()?;
    pam.credentials(CredentialsOP::Delete)?;
    pam.end()?;

    Ok(())
}

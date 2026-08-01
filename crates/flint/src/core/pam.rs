use anyhow::{Context, Result};
use flint_pam::*;

use crate::seat::view::View;

struct PamSession {
    pam: Pam,
}

impl PamSession {
    fn start(
        env: impl envy::Diff,
        username: Option<&str>,
        display: Option<impl PamDisplay>,
        require_auth: bool,
    ) -> Result<Self> {
        let mut pam = Pam::new("flint", display, username)?;

        if require_auth {
            pam.authenticate(false)?;
        }
        pam.assert_account_is_valid(false)?;
        pam.credentials(CredentialsOP::Establish)?;

        pam.set_env(env)?;
        pam.open_session()?;

        Ok(Self { pam })
    }

    fn username(&mut self) -> Result<String> {
        self.pam.get_username()
    }

    fn view(&self) -> Result<View> {
        // TODO: this should never be necessary under new model
        View::from_env(&self.pam)
            .context("Could not get seat/vt from PAM env. Check if systemd_pam is in the stack.")
    }
}

impl Drop for PamSession {
    fn drop(&mut self) {
        self.pam.close_session().unwrap();
        self.pam.credentials(CredentialsOP::Delete).unwrap();
    }
}

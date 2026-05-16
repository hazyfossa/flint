mod bind;
mod environment;
mod utils;
mod worker;

use std::path::PathBuf;

use anyhow::Result;
use argh::FromArgs;
use envy::Get;
use rustix::process;
use serde::{Deserialize, Serialize};

use crate::{
    bind::pam::{CredentialsOP, Pam, PamDisplay},
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
    vt: Option<VtNumber>,
    seat: Option<Seat>,
}

struct Session {
    pam: Pam,
}

impl Session {
    fn start(
        env: impl envy::Diff,
        username: Option<&str>,
        display: impl PamDisplay,
        require_auth: bool,
        silent: bool,
    ) -> Result<Self> {
        let mut pam = Pam::new("flint", display, username, silent)?;

        if require_auth {
            pam.authenticate(false)?;
        }
        pam.assert_account_is_valid(false)?;
        pam.credentials(CredentialsOP::Establish)?;

        pam.set_env(env);
        pam.open_session()?;

        Ok(Self { pam })
    }

    fn view(&self) -> View {
        View {
            vt: self.pam.get::<VtNumber>().ok(),
            seat: self.pam.get::<Seat>().ok(),
        }
    }

    fn end(mut self) {
        // TODO: log errors
        let _ = self.pam.close_session();
        let _ = self.pam.credentials(CredentialsOP::Delete);
        self.pam.end();
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber)?;

    let args: Args = argh::from_env();

    Ok(())
}

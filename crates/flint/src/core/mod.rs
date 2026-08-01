mod pam;

use std::{os::fd::AsFd, path::PathBuf};

use anyhow::{Context, Result};
use envy::{define_env, parse::EnvironmentParse};
use serde::{Deserialize, Serialize};

use crate::utils::tty::Terminal;

#[derive(Serialize, Deserialize)]
pub enum Runtime {
    // run like systemd does not exist
    Unix,

    // activate systemd, but do not integrate
    // mirrors behaviour of some other session managers
    // not recommended to use, except for compatibility
    Split,

    // pull environment from systemd, provide unit shims
    Hybrid,

    // run as transient units
    Systemd,
}

#[derive(Serialize, Deserialize)]
pub enum Target {
    Unit {
        name: String,
    },
    #[serde(untagged)]
    Command {
        executable: PathBuf,
        runtime: Option<Runtime>, // TODO: default is configured externally
    },
}

// TODO: make Default of Option<> easily resolvable globally
// ref: make Config into OnceCell

// impl Target {
//     fn activates_systemd(&self) -> bool {
//         match self {
//             Self::Unit { .. } => true,
//             Self::Command { runtime, .. } => !matches!(runtime, Runtime::Unix),
//         }
//     }

//     fn pulls_env_from_systemd(&self) -> bool {
//         match self {
//             // there is no need to
//             Self::Unit { .. } => false,
//             // all others either do not care, or already have the env
//             Self::Command { runtime, .. } => matches!(runtime, Runtime::Hybrid),
//         }
//     }
// }

enum Kind {
    Text,
    Graphical { primary: bool },
}

struct Session {
    target: Target,
    kind: Kind,
}

// TODO: we are mixing two username flows at the moment.
// f1: we ask for username -> we resolve -> we pass to pam -> pam asks for pass
// f2: pam asks for username + pass -> pam resolves -> we ask pam -> we double-resolve??

// UserIncomplete, Manager, Background and None are not here as those aren't relevant
#[allow(dead_code)]
pub enum SessionClass {
    User { early: bool, light: bool },
    Greeter,
    LockScreen,
}

impl SessionClass {
    fn user_default() -> Self {
        Self::User {
            early: false,
            light: false,
        }
    }
}

define_env!(SessionClass = #custom "XDG_SESSION_CLASS");

impl EnvironmentParse<String> for SessionClass {
    type Error = std::convert::Infallible;

    fn env_serialize(self) -> String {
        match self {
            Self::User { early, light } => {
                let mut string = "user".to_string();
                if early {
                    string += "-early"
                }
                if light {
                    string += "-light"
                }
                string
            }
            Self::Greeter => "greeter".to_string(),
            Self::LockScreen => "lock-screen".to_string(),
        }
    }

    fn env_deserialize(_value: String) -> Result<Self, Self::Error> {
        todo!()
    }
}

// TODO: does pam do this for us?
fn new_shell_session<F: AsFd>(ctty: &Terminal<F>) -> Result<()> {
    rustix::process::setsid().context("Failed to create a new process-tree session (setsid)")?;

    ctty.set_as_ctty()
        .context("Failed to set controlling tty")?;

    Ok(())
}

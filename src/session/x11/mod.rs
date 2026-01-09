mod auth;

use anyhow::{Context, Result, anyhow};
use facet::Facet;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    net::unix::pipe,
    process::Command,
};

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use auth::XAuthorityManager;

use crate::{
    environment::prelude::*,
    session::{SessionType, manager::SessionContext, metadata::FreedesktopMetadata},
    utils::{
        fd::{CommandFdCtxExt, FdContext},
        misc::OsStringExt,
    },
};

static DEFAULT_XORG_PATH: &str = "/usr/lib/Xorg";

define_env!("DISPLAY", pub Display(u8));

impl Display {
    pub fn new(number: u8) -> Self {
        Self(number)
    }

    pub fn number(&self) -> u8 {
        self.0
    }
}

impl EnvParser for Display {
    fn serialize(&self) -> OsString {
        format!(":{}", self.0).into()
    }

    fn deserialize(value: OsString) -> Result<Self> {
        Ok(Self(
            value
                .try_to_string()?
                .strip_prefix(":")
                .ok_or(anyhow!("display should start with :"))?
                .parse()?,
        ))
    }
}

struct DisplayReceiver(pipe::Receiver);

impl DisplayReceiver {
    fn setup<'a>(fd_ctx: &mut FdContext, command: &'a mut Command) -> Result<Self> {
        let (display_tx, display_rx) =
            pipe::pipe().context("Failed to open pipe for display fd")?;

        let display_tx_passed = fd_ctx.pass(display_tx.into_blocking_fd()?)?;

        command.args(["-displayfd", &display_tx_passed.num().to_string()]);

        Ok(Self(display_rx))
    }

    async fn display(self) -> Result<Display> {
        let mut reader = BufReader::new(self.0);
        let mut display_buf = String::new();

        reader
            .read_line(&mut display_buf)
            .await
            .context("Failed to read display number")?;

        if display_buf.is_empty() {
            Err(anyhow!("Internal Xorg error. See logs above for details."))
        } else {
            Ok(Display::new(
                display_buf
                    .trim_end()
                    .parse()
                    .context("Xorg provided invalid display number")?,
            ))
        }
    }
}

// TODO: is this relevant for modern systems?
// If yes, we'll need to do VT allocation before xorg
// define_env!("WINDOWPATH", pub WindowPath(String));
// env_parser_auto!(WindowPath);

// impl WindowPath {
//     fn previous_plus_vt(env: &Env, vt: &VtNumber) -> Result<Self> {
//         let previous = env.peek::<Self>();
//         Ok(Self(match previous {
//             Ok(path) => format!("{}:{}", *path, vt.to_string()),
//             Err(_) => vt.to_string(),
//         }))
//     }
// }

#[derive(Facet)]
#[facet(default)]
pub struct SessionConfig {
    xorg_path: PathBuf,
    lock_authority: bool,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            xorg_path: PathBuf::from(DEFAULT_XORG_PATH),
            lock_authority: true,
        }
    }
}

impl FreedesktopMetadata for SessionConfig {
    const LOOKUP_PATH: &str = "/usr/share/xsessions";
}

fn spawn_server(
    path: &Path,
    authority: PathBuf,
    context: &SessionContext,
) -> Result<DisplayReceiver> {
    let mut fd_ctx = FdContext::new(3..5);

    let mut command = Command::new(path);

    if let Some(vt) = &context.vt {
        command.arg(format!("vt{}", vt.to_string()));
    }

    command
        .args(["-seat".into(), context.seat.serialize()])
        .args(["-auth".into(), authority.into_os_string()])
        .args(["-nolisten", "tcp"])
        .args(["-background", "none", "-noreset", "-keeptty", "-novtswitch"])
        .args(["-verbose", "3", "-logfile", "/dev/null"])
        .envs([("XORG_RUN_AS_USER_OK", "1")]); // TODO: relevant?

    let display_rx = DisplayReceiver::setup(&mut fd_ctx, &mut command)?;
    command.with_fd_context(fd_ctx);

    context.spawn(command)?;

    Ok(display_rx)
}

#[async_trait::async_trait]
impl SessionType for SessionConfig {
    fn tag(&self) -> &'static str {
        "x11"
    }

    async fn setup_session(&self, context: &mut SessionContext, executable: &Path) -> Result<()> {
        // let window_path = WindowPath::previous_plus_vt(&context.env, &context.terminal.number)?;

        let authority_manager = XAuthorityManager::new(context, self.lock_authority)
            .context("Failed to setup authority manager")?;

        let server_authority = authority_manager
            .setup_server()
            .context("Failed to define server authority")?;

        let server = spawn_server(&self.xorg_path, server_authority, context)?;

        // NOTE: this will block until the X server is ready
        let display = server.display().await?;

        let client_authority = authority_manager
            .setup_client(&display)
            .context("failed to define client authority")?;

        authority_manager.finish(context)?;

        context.update_env((display, client_authority));
        context.spawn(Command::new(executable))
    }
}

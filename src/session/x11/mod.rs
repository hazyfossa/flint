mod auth;

use anyhow::{Context, Result, anyhow};

use std::{
    ffi::OsString,
    io::{BufRead, BufReader, PipeReader, pipe},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::mpsc,
    thread::{self, JoinHandle},
};

use super::manager::prelude::*;

use auth::{ClientAuthorityEnv, XAuthorityManager};

use crate::{
    environment::prelude::*,
    login::context::ExitReason,
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

struct DisplayReceiver(PipeReader);

impl DisplayReceiver {
    fn setup<'a>(
        fd_ctx: &mut FdContext,
        command: &'a mut Command,
    ) -> Result<(Self, &'a mut Command)> {
        let (display_rx, display_tx) = pipe().context("Failed to open pipe for display fd")?;
        let display_tx_passed = fd_ctx.pass(display_tx.into())?;

        let command = command.args(["-displayfd", &display_tx_passed.num().to_string()]);

        Ok((Self(display_rx), command))
    }

    fn wait(self) -> Result<Option<Display>> {
        let mut reader = BufReader::new(self.0);
        let mut display_buf = String::new();

        reader
            .read_line(&mut display_buf)
            .context("Failed to read display number")?;

        let display = if display_buf.is_empty() {
            None
        } else {
            Some(Display::new(
                display_buf
                    .trim_end()
                    .parse()
                    .context("Xorg provided invalid display number")?,
            ))
        };

        Ok(display)
    }
}

define_env!("WINDOWPATH", pub WindowPath(String));
env_parser_auto!(WindowPath);

impl WindowPath {
    fn previous_plus_vt(env: &Env, vt: &VtNumber) -> Result<Self> {
        let previous = env.peek::<Self>();
        Ok(Self(match previous {
            Ok(path) => format!("{}:{}", path.0, vt.to_string()),
            Err(_) => vt.to_string(),
        }))
    }
}

pub struct XWatcher {
    process: Child,
    shutdown_tx: mpsc::Sender<ExitReason>,
}

impl XWatcher {
    fn log(line: String) {
        println!("Xorg: {line}") // TODO: log levels
    }

    fn handler(mut self) {
        let mut stdout = BufReader::new(
            self.process
                .stderr
                .take()
                .expect("Xorg stderr not piped somehow"),
        )
        .lines();

        while let Some(Ok(line)) = stdout.next() {
            Self::log(line);
        }

        let exit_status = self
            .process
            .wait()
            .expect("Xorg not started, yet attaching logger");

        self.shutdown_tx
            .send(format!("Xorg exited with {exit_status}"))
            .expect("Cannot send clean shutdown signal, aborting");
    }

    fn start_thread(self) -> Result<JoinHandle<()>> {
        thread::Builder::new()
            .name("Xorg watcher".into())
            .spawn(|| self.handler())
            .context("Failed to start logger thread")
    }
}

pub struct Session;

impl metadata::FreedesktopMetadata for Session {
    const LOOKUP_PATH: &str = "/usr/share/xsessions";
}

#[derive(Deserialize)]
#[serde(default)]
pub struct Config {
    xorg_path: PathBuf,
    lock_authority: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            xorg_path: PathBuf::from(DEFAULT_XORG_PATH),
            lock_authority: true,
        }
    }
}

fn spawn_server(
    path: &Path,
    authority: PathBuf,
    context: &SessionContext,
) -> Result<(DisplayReceiver, Child)> {
    let mut fd_ctx = FdContext::new(3..5);

    // TODO: this is flaky. Unsetting env causes strange behaviour.
    // Ensure that Xorg always starts non-elevated or bypass Xorg.wrap entirely
    let mut command = context.command(path);
    command
        .arg(format!("vt{}", context.vt.to_string()))
        .args(["-seat".into(), context.seat.serialize()])
        .args(["-auth".into(), authority.into_os_string()])
        .args(["-nolisten", "tcp"])
        .args(["-background", "none", "-noreset", "-keeptty", "-novtswitch"])
        .args(["-verbose", "3", "-logfile", "/dev/null"])
        .envs([("XORG_RUN_AS_USER_OK", "1")]); // TODO: relevant?

    let (display_rx, command) = DisplayReceiver::setup(&mut fd_ctx, &mut command)?;

    let process = command
        .with_fd_context(fd_ctx)
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn X server subprocess")?;

    Ok((display_rx, process))
}

impl manager::SessionType for Session {
    const XDG_TYPE: &str = "x11";

    type ManagerConfig = Config;
    type EnvDiff = (Display, ClientAuthorityEnv, WindowPath);

    fn setup_session(config: &Config, context: &mut SessionContext) -> Result<Self::EnvDiff> {
        let window_path = WindowPath::previous_plus_vt(&context.env, &context.vt)?;

        let authority_manager = XAuthorityManager::new(context, config.lock_authority)
            .context("Failed to setup authority manager")?;

        let server_authority = authority_manager
            .setup_server()
            .context("Failed to define server authority")?;

        let (future_display, process) = spawn_server(&config.xorg_path, server_authority, context)?;

        XWatcher {
            process,
            shutdown_tx: context.shutdown_tx.clone(),
        }
        .start_thread()?;

        // NOTE: this will block until the X server is ready
        if let Some(display) = future_display.wait()? {
            let client_authority = authority_manager
                .setup_client(&display)
                .context("failed to define client authority")?;

            Ok((display, client_authority, window_path))
        } else {
            Err(anyhow!("Internal Xorg error. See logs above for details."))
        }
    }
}

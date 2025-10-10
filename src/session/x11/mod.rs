mod auth;

use anyhow::{Context, Result, anyhow};

use std::{
    ffi::OsString,
    io::{BufRead, BufReader, PipeReader, pipe},
    path::PathBuf,
    process::{self, Child, Command, Stdio},
    thread::{self, JoinHandle},
};

use super::{context::VtNumber, manager::prelude::*};

use auth::{ClientAuthorityEnv, XAuthorityManager};

use crate::{
    environment::{Env, EnvValue},
    utils::{
        fd::{CommandFdCtxExt, FdContext},
        misc::OsStringExt,
    },
};

static DEFAULT_XORG_PATH: &str = "/usr/lib/Xorg";

pub struct Display(u8);

impl Display {
    pub fn new(number: u8) -> Self {
        Self(number)
    }

    pub fn number(&self) -> u8 {
        self.0
    }
}

impl EnvValue for Display {
    const KEY: &str = "DISPLAY";

    fn serialize(self) -> OsString {
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

crate::define_env!("WINDOWPATH", pub WindowPath(String));

impl WindowPath {
    fn previous_plus_vt(env: &Env, vt: &VtNumber) -> Result<Self> {
        let previous = env.peek::<Self>()?;
        Ok(Self(match previous {
            Some(path) => format!("{}:{}", path.0, vt.to_string()),
            None => vt.to_string(),
        }))
    }
}

pub struct XWatcher {
    process: Child,
}

impl XWatcher {
    fn log(line: String) {
        println!("Xorg: {line}") // TODO: log levels
    }

    fn handler(mut self) -> process::ExitStatus {
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

        self.process
            .wait()
            .expect("Xorg not started, yet attaching logger")
    }

    fn start_thread(self) -> Result<JoinHandle<process::ExitStatus>> {
        thread::Builder::new()
            .name("Xorg watcher".into())
            .spawn(|| self.handler())
            .context("Failed to start logger thread")
    }
}

#[derive(Deserialize)]
#[serde(default)]
pub struct SessionManager {
    xorg_path: PathBuf,
    lock_authority: bool,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self {
            xorg_path: PathBuf::from(DEFAULT_XORG_PATH),
            lock_authority: true,
        }
    }
}

impl FreedesktopMetadata for SessionManager {
    const LOOKUP_PATH: &str = "/usr/share/xsessions";
}

impl SessionManager {
    fn spawn_server(
        &self,
        authority: PathBuf,
        context: SessionContext,
    ) -> Result<(DisplayReceiver, Child)> {
        let mut fd_ctx = FdContext::new(3..5);

        let mut xorg_path = Command::new(&self.xorg_path);

        // TODO: this is flaky. Unsetting env causes strange behaviour.
        // Ensure that Xorg always starts non-elevated or bypass Xorg.wrap entirely
        let command = xorg_path
            .arg(format!("vt{}", context.vt.to_string()))
            .args(["-seat".into(), context.seat.serialize()])
            .args(["-auth".into(), authority.into_os_string()])
            .args(["-nolisten", "tcp"])
            .args(["-background", "none", "-noreset", "-keeptty", "-novtswitch"])
            .args(["-verbose", "3", "-logfile", "/dev/null"])
            .envs([("XORG_RUN_AS_USER_OK", "1")]); // TODO: relevant?

        let (display_rx, command) = DisplayReceiver::setup(&mut fd_ctx, command)?;

        let process = command
            .with_fd_context(fd_ctx)
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn X server subprocess")?;

        Ok((display_rx, process))
    }
}

impl manager::SessionManager for SessionManager {
    const XDG_TYPE: &str = "x11";

    type Env = (Display, ClientAuthorityEnv, WindowPath);

    fn setup_session(&self, context: SessionContext) -> Result<Self::Env> {
        let env = &context.inherit_env;
        let window_path = WindowPath::previous_plus_vt(env, &context.vt)?;

        let authority_manager = XAuthorityManager::new(&context.vt, self.lock_authority)
            .context("Failed to setup authority manager")?;

        let server_authority = authority_manager
            .setup_server()
            .context("Failed to define server authority")?;

        let (future_display, process) = self.spawn_server(server_authority, context)?;
        XWatcher { process }.start_thread()?; // TODO: requires changes to trait

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

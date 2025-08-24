mod auth;

mod connection;
mod resources;

use anyhow::{Context, Result, anyhow};
use auth::XAuthorityManager;

use std::{
    ffi::OsString,
    io::{BufRead, BufReader, PipeReader, pipe},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread::{self, JoinHandle},
};

use crate::{
    Seat,
    console::VtNumber,
    environment::{EnvDiff, EnvValue},
    template,
    utils::{
        fd::{CommandFdCtxExt, FdContext},
        misc::OsStringExt,
        runtime_dir::RuntimeDir,
        subprocess::{CommandLifetimeExt, ExitStatusExt, Ret},
    },
    x::auth::ClientAuthorityEnv,
};

pub struct Display(u8);

impl Display {
    pub fn new(number: u8) -> Self {
        Self(number)
    }

    pub fn number(&self) -> u8 {
        self.0
    }

    pub fn local_socket(&self) -> String {
        format!("/tmp/.X11-unix/X{}", self.0)
    }
}

impl EnvValue for Display {
    const KEY: &str = "DISPLAY";

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

struct WindowPath(String);

impl EnvValue for WindowPath {
    const KEY: &str = "WINDOWPATH";
    crate::env_impl!();
}

impl WindowPath {
    fn previous_plus_vt(vt: &VtNumber) -> Self {
        let previous = Self::current().ok();
        Self(match previous {
            Some(path) => format!("{}:{vt}", path.0),
            None => vt.to_string(),
        })
    }
}

pub struct XWatcher {
    process: Child,
}

impl XWatcher {
    fn log(line: String) {
        println!("Xorg: {line}") // TODO: log levels
    }

    fn handler(mut self) -> Ret {
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
            .context_process_name("Xorg".into())
    }

    fn start_thread(self) -> Result<JoinHandle<Ret>> {
        thread::Builder::new()
            .name("Xorg watcher".into())
            .spawn(|| self.handler())
            .context("Failed to start logger thread")
    }
}

fn spawn_server(
    path: &Path,
    authority: PathBuf,
    vt: VtNumber,
    seat: Seat,
) -> Result<(DisplayReceiver, Child)> {
    let mut fd_ctx = FdContext::new(3..5);

    let mut xorg_path = Command::new(path);

    // TODO: this is flaky. Unsetting env causes strange behaviour.
    // Ensure that Xorg always starts non-elevated or bypass Xorg.wrap entirely
    let command = xorg_path
        .arg(format!("vt{vt}"))
        .args(["-seat".into(), seat.serialize()])
        .args(["-auth".into(), authority.into_os_string()])
        .args(["-nolisten", "tcp"])
        .args(["-background", "none", "-noreset", "-keeptty", "-novtswitch"])
        .args(["-verbose", "3", "-logfile", "/dev/null"])
        .envs([("XORG_RUN_AS_USER_OK", "1")]) // TODO: relevant?
        .bind_lifetime();

    let (display_rx, command) = DisplayReceiver::setup(&mut fd_ctx, command)?;

    let process = command
        .with_fd_context(fd_ctx)
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn subprocess")?;

    Ok((display_rx, process))
}

pub struct Session {
    display: Display,
    client_authority: ClientAuthorityEnv,
    window_path: WindowPath,
}

impl template::Session for Session {
    const LOOKUP_PATH: &str = "/usr/share/xsessions";
    const XDG_TYPE: &str = "x11";

    fn env(self) -> EnvDiff {
        EnvDiff::build()
            .set(self.display)
            .set(self.client_authority)
            .set(self.window_path)
            .seal()
    }
}

pub struct SessionManager {
    // TODO: config
    xorg_path: PathBuf,
    runtime_dir: RuntimeDir,
}

impl SessionManager {
    pub fn with_config(xorg_path: PathBuf, runtime_dir: RuntimeDir) -> Self {
        Self {
            xorg_path,
            runtime_dir,
        }
    }
}

impl template::SessionManager<Session> for SessionManager {
    fn setup(self) -> Result<Session> {
        let vt = VtNumber::current().context("VT not allocated or XDG_VTNR is unset")?;
        let seat = Seat::current().unwrap_or_default();

        let window_path = WindowPath::previous_plus_vt(&vt);

        // TODO: locking config
        let authority_manager = XAuthorityManager::new(&self.runtime_dir, &vt, true)
            .context("Failed to setup authority manager")?;

        let server_authority = authority_manager
            .setup_server()
            .context("Failed to define server authority")?;

        let (future_display, process) = spawn_server(&self.xorg_path, server_authority, vt, seat)?;
        let watcher = XWatcher { process }.start_thread()?; // TODO: requires changes to trait

        // NOTE: this will block until the X server is ready
        if let Some(display) = future_display.wait()? {
            let client_authority = authority_manager
                .setup_client(&display)
                .context("failed to define client authority")?;

            let session = Session {
                display,
                client_authority,
                window_path,
            };

            Ok(session)
        } else {
            Err(anyhow!("Internal Xorg error. See logs above for details."))
        }
    }
}

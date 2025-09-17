pub mod misc {
    use std::{ffi::OsString, io};

    pub trait OsStringExt {
        fn try_to_string(self) -> io::Result<String>;
    }

    impl OsStringExt for OsString {
        fn try_to_string(self) -> io::Result<String> {
            self.into_string().map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "String is not valid unicode (UTF-8)",
                )
            })
        }
    }
}

pub mod fd {
    use std::{ops::Range, os::fd::OwnedFd, path::PathBuf, process::Command};

    use anyhow::{Result, anyhow};
    use command_fds::{CommandFdExt, FdMapping};

    // TODO: allow any iterator of u32 as fd source to support non-continuous definitions
    // low priority
    pub struct FdContext {
        free_fd_source: Range<u32>,
        mappings: Vec<FdMapping>,
    }

    impl FdContext {
        pub fn new(free_fd_source: Range<u32>) -> Self {
            Self {
                free_fd_source,
                mappings: Vec::new(),
            }
        }

        pub fn pass(&mut self, fd: OwnedFd) -> Result<PassedFd> {
            let mapped_fd = self
                .free_fd_source
                .next()
                .ok_or(anyhow!("Free fd source exhausted"))?;

            self.mappings.push(FdMapping {
                parent_fd: fd,
                child_fd: mapped_fd as i32, // TODO: why signed here? Fds are positive-only
            });
            Ok(PassedFd(mapped_fd))
        }
    }

    pub trait CommandFdCtxExt: CommandFdExt {
        fn with_fd_context(&mut self, fd_ctx: FdContext) -> &mut Self;
    }

    impl CommandFdCtxExt for Command {
        fn with_fd_context(&mut self, fd_ctx: FdContext) -> &mut Self {
            self.fd_mappings(fd_ctx.mappings)
                .expect("fd context generated invalid mappings") // TODO: is this a safe assumtion?
        }
    }

    pub struct PassedFd(u32);

    impl PassedFd {
        pub fn path(&self) -> PathBuf {
            PathBuf::from("/proc/self/fd/").join(self.0.to_string())
        }

        pub fn num(&self) -> u32 {
            self.0
        }
    }
}

pub mod timer {
    use std::time::{Duration, Instant};

    pub struct Timer {
        started: Instant,
    }

    impl Timer {
        pub fn start() -> Self {
            Self {
                started: Instant::now(),
            }
        }

        pub fn measure(&self) -> Duration {
            let now = Instant::now();
            now - self.started
        }
    }
}

pub mod runtime_dir {
    use std::{
        fs,
        ops::Deref,
        os::unix::fs::DirBuilderExt,
        path::{Path, PathBuf},
        sync::OnceLock,
    };

    use anyhow::{Context, Result};

    // TODO: clean error propagation. Possibly abandon lazy loading at all.
    static RUNTIME_DIR: OnceLock<RuntimeDir> = OnceLock::new();

    pub fn current() -> &'static RuntimeDir {
        RUNTIME_DIR.get_or_init(|| {
            let xdg_context = xdg::BaseDirectories::new();
            RuntimeDir::create(&xdg_context, "troglodyt")
                .expect("Error while creating runtime directory")
        })
    }

    #[derive(Debug)]
    pub struct RuntimeDir {
        path: PathBuf,
    }

    impl RuntimeDir {
        pub fn create(xdg_context: &xdg::BaseDirectories, app_name: &str) -> Result<Self> {
            let path = xdg_context
                .get_runtime_directory()
                .context("Failed to query base runtime directory")?
                .join(app_name);

            fs::DirBuilder::new()
                .mode(0o700)
                .recursive(true)
                .create(&path)?;

            Ok(Self { path })
        }
    }

    impl Deref for RuntimeDir {
        type Target = Path;

        fn deref(&self) -> &Self::Target {
            &self.path
        }
    }

    impl Drop for RuntimeDir {
        fn drop(&mut self) {
            _ = fs::remove_dir(self.path.clone())
        }
    }
}

pub mod subprocess {
    use std::{
        os::unix::process::CommandExt,
        process::{Command, ExitStatus},
    };

    use anyhow::{Result, anyhow};
    use rustix::process::{self, Signal};

    // TODO: This can probably be done MUCH better with proper errors in place of anyhow

    pub struct Ret {
        code: Option<i32>,
        process_name: String,
    }

    impl Ret {
        pub fn ok(self) -> Result<()> {
            let process_name = self.process_name;

            match self.code {
                Some(0) => Ok(()),
                Some(err_status) => {
                    Err(anyhow!("{process_name} exited with status: {err_status}."))
                }
                None => Err(anyhow!("{process_name} terminated by signal.")),
            }
        }
    }

    pub trait ExitStatusExt {
        fn context_process_name(self, process_name: String) -> Ret;
    }

    impl ExitStatusExt for ExitStatus {
        fn context_process_name(self, process_name: String) -> Ret {
            Ret {
                code: self.code(),
                process_name,
            }
        }
    }

    pub trait CommandLifetimeExt {
        fn bind_lifetime(&mut self) -> &mut Self;
    }

    impl CommandLifetimeExt for Command {
        fn bind_lifetime(&mut self) -> &mut Self {
            // TODO: is the safety of rustix enough here?
            unsafe {
                self.pre_exec(|| {
                    Ok(process::set_parent_process_death_signal(Some(
                        Signal::KILL,
                    ))?)
                });
            }
            self
        }
    }
}

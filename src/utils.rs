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
    use std::{ops::Range, os::fd::OwnedFd, process::Command};

    use anyhow::{Result, anyhow};
    use command_fds::{CommandFdExt, FdMapping};

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
            self.fd_mappings(fd_ctx.mappings).expect(
                "Fd collision at context detected at runtime.
                Check if any manual mappings overlap with free_fd_source.",
            )
        }
    }

    pub struct PassedFd(u32);

    impl PassedFd {
        // pub fn path(&self) -> PathBuf {
        //     PathBuf::from("/proc/self/fd/").join(self.0.to_string())
        // }

        pub fn num(&self) -> u32 {
            self.0
        }
    }
}

pub mod globals {
    use std::sync::OnceLock;

    use anyhow::{Result, anyhow};

    pub struct Global<T> {
        inner: OnceLock<T>,
        name: &'static str,
    }

    impl<T> Global<T> {
        pub const fn define(name: &'static str) -> Self {
            Self {
                inner: OnceLock::new(),
                name,
            }
        }

        pub fn get(&self) -> Result<&T> {
            self.inner
                .get()
                .ok_or(anyhow!("Global {} not initialized", self.name))
        }

        pub fn init(&self, object: T) -> Result<()> {
            self.inner
                .set(object)
                .map_err(|_| anyhow!("Cannot initialize global {} twice", self.name))
        }
    }
}

pub mod runtime_dir {
    use std::{
        fs,
        ops::Deref,
        os::unix::fs::DirBuilderExt,
        path::{Path, PathBuf},
    };

    use anyhow::{Context, Result};

    use crate::utils::globals::Global;

    #[allow(non_upper_case_globals)]
    pub static current: Global<RuntimeDir> = Global::define("runtime dir");

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
}

mod macro_scope {
    // See https://github.com/rust-lang/rust/issues/35853#issuecomment-415993963
    #[macro_export]
    macro_rules! scope {
    ($($body:tt)*) => {
        macro_rules! __with_dollar_sign { $($body)* }
        __with_dollar_sign!($);
        }
    }
}

pub mod config {
    use std::{
        collections::HashMap,
        convert::Infallible,
        io::{ErrorKind, Read},
        path::PathBuf,
    };

    use anyhow::{Context, Result};
    use fs_err::File;
    use pico_args::Arguments;
    use serde::Deserialize;
    use toml::Table;

    type Partial = Table;

    #[derive(Deserialize, Default, Debug)]
    pub struct Config {
        #[allow(dead_code)]
        version: Option<String>,
        pub session: HashMap<String, Partial>,
    }

    impl Config {
        pub fn from_args(args: &mut Arguments, default_path: &str) -> Result<Self> {
            let config_path = args
                .opt_value_from_os_str::<_, _, Infallible>(["-c", "--config"], |path| {
                    Ok(PathBuf::from(path))
                })?
                .unwrap_or(PathBuf::from(default_path));

            Self::from_file(config_path).context("Failed to read config")
        }

        fn from_file(path: PathBuf) -> Result<Self> {
            let mut file = match File::open(&path) {
                Err(e) if matches!(e.kind(), ErrorKind::NotFound) => return Ok(Self::default()),
                other => other,
            }?;

            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;

            Ok(toml::from_slice(&buf).context("Invalid config")?)
        }
    }
}

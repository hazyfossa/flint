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
    use std::{ops::Range, os::fd::OwnedFd};

    use anyhow::{Result, anyhow};
    use command_fds::{CommandFdExt, FdMapping};
    use tokio::process::Command;

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
                child_fd: mapped_fd as i32,
            });
            Ok(PassedFd(mapped_fd))
        }
    }

    pub trait CommandFdCtxExt: CommandFdExt {
        fn with_fd_context(&mut self, fd_ctx: FdContext) -> &mut Self;
    }

    impl CommandFdCtxExt for Command {
        fn with_fd_context(&mut self, fd_ctx: FdContext) -> &mut Self {
            // if you see this error,
            // check if any manual mappings overlap with free_fd_source.
            self.fd_mappings(fd_ctx.mappings)
                .expect("Fd collision with context detected at runtime.")
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

pub mod runtime_dir {
    use std::{
        fs::{self, DirBuilder, remove_dir_all},
        os::unix::fs::{DirBuilderExt, PermissionsExt},
        path::PathBuf,
    };

    use anyhow::{Context, Result, anyhow};
    use shrinkwraprs::Shrinkwrap;

    use crate::{APP_NAME, environment::prelude::*};

    #[derive(Shrinkwrap)]
    pub struct RuntimeDir {
        #[shrinkwrap(main_field)]
        path: PathBuf,
    }

    impl Drop for RuntimeDir {
        fn drop(&mut self) {
            let _ = remove_dir_all(&self.path);
        }
    }

    #[derive(Debug)]
    pub struct RuntimeDirManager {
        path: PathBuf,
    }

    define_env!("XDG_RUNTIME_DIR", RuntimeDirEnv(PathBuf));
    env_parser_raw!(RuntimeDirEnv);

    impl RuntimeDirManager {
        pub fn from_env(env: &Env) -> Result<Self> {
            let path = &*env
                .peek::<RuntimeDirEnv>()
                .context("Environment does not provide a runtime directory")?;

            let permissions = fs_err::metadata(&path)?.permissions().mode();

            if permissions & 0o077 != 0 {
                Err(anyhow!(
                    "Runtime directory is insecure: expecting permissions `077`, got {permissions}"
                ))
            } else {
                Self::new(path.to_path_buf())
            }
        }

        fn new(path: PathBuf) -> Result<Self> {
            fs::DirBuilder::new()
                .mode(0o700)
                .recursive(true)
                .create(&path.join(APP_NAME))?;

            Ok(Self { path })
        }

        pub fn create(&self, name: &str) -> Result<RuntimeDir> {
            let directory = self.path.join(name);

            DirBuilder::new()
                .mode(0o700)
                .create(&directory)
                .context(format!("cannot create path: {directory:?}"))?;

            Ok(RuntimeDir { path: directory })
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

pub mod bufio {
    use anyhow::Result;
    use binrw::{
        BinRead, BinWrite,
        io::NoSeek,
        meta::{ReadEndian, WriteEndian},
    };
    pub use bytes::{Buf, BufMut};

    pub trait BufRead: Sized {
        fn read_buf(buf: &mut impl Buf) -> Result<Self>;
    }

    #[allow(dead_code)]
    pub trait BufWrite {
        fn write_buf(&self, buf: &mut impl BufMut) -> Result<()>;
    }

    // NOTE: this implementation effectively prohibits using Seek'ing features of binrw
    impl<T> BufRead for T
    where
        T: BinRead + ReadEndian,
        for<'a> <T as BinRead>::Args<'a>: Default,
    {
        fn read_buf(buf: &mut impl Buf) -> Result<Self> {
            Ok(Self::read(&mut NoSeek::new(buf.reader()))?)
        }
    }

    impl<T> BufWrite for T
    where
        T: BinWrite + WriteEndian,
        for<'a> <T as BinWrite>::Args<'a>: Default,
    {
        fn write_buf(&self, buf: &mut impl BufMut) -> Result<()> {
            Ok(self.write(&mut NoSeek::new(buf.writer()))?)
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
    use facet::Facet;
    use facet_value::Value;
    use fs_err::File;
    use pico_args::Arguments;

    use crate::{
        greet::GreeterConfig, mode::daemon::DaemonConfig, session::metadata::SessionMetadataMap,
    };

    #[derive(Facet, Default, Clone)]
    pub struct SessionTypeConfig {
        #[facet(flatten)]
        pub config: Value,
        #[facet(rename = "session")]
        pub entries: SessionMetadataMap,
    }

    #[derive(Facet, Default)]
    pub struct Config {
        #[allow(dead_code)]
        version: Option<String>,
        #[facet(rename = "greeter")]
        pub greeters: HashMap<String, GreeterConfig>,
        #[facet(flatten)]
        pub sessions: HashMap<String, SessionTypeConfig>,
        #[facet(default)]
        pub daemon: Option<DaemonConfig>,
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

            Ok(facet_kdl::from_slice(&buf)?)
        }
    }
}

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
        io::{ErrorKind, Read},
        path::PathBuf,
    };

    use anyhow::Result;
    use fs_err::File;
    use serde::de::DeserializeOwned;

    pub fn config_from_file<T: DeserializeOwned + Default>(path: PathBuf) -> Result<T> {
        let mut file = match File::open(&path) {
            Err(e) if matches!(e.kind(), ErrorKind::NotFound) => return Ok(T::default()),
            other => other,
        }?;

        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;

        Ok(toml::from_slice(&buf)?)
    }
}

pub mod macros {
    #[macro_export]
    macro_rules! trait_alias {
        ($vis:vis trait $name:ident = $($for:tt)*) => {
            $vis trait $name: $($for)* {}
            impl<T: $($for)*> $name for T {}
        };
    }

    // See https://github.com/rust-lang/rust/issues/35853#issuecomment-415993963
    #[macro_export]
    macro_rules! scope {
    ($($body:tt)*) => {
        macro_rules! __with_dollar_sign { $($body)* }
        __with_dollar_sign!($);
        }
    }
}

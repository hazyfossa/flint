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

    #[macro_export]
    macro_rules! with_builder {
    (
        $vis:vis struct $name:ident {
            $($fvis:vis $key:ident : $(#$kind:meta)? $value:path,)*
        }
    ) => { paste::paste! {
        $vis struct $name {
            $($fvis $key : $crate::with_builder!(@repr $($kind)? $value),)*
        }

        struct [<$name Builder>] {
            $($key: Option<$value>,)*
        }

        impl [<$name Builder>] {
            fn new() -> Self {
                Self {$( $key: None, )*}
            }

            $(
                fn [<set_ $key:lower>](&mut self, value: $value) -> &mut Self {
                    self.$key = self.$key.replace(value);
                    self
                }
            )*

            fn finalize(self) -> anyhow::Result<$name> {
                use anyhow::Context;
                Ok($name {$(
                    $key: $crate::with_builder!(@finalize $($kind)? self.$key),
                )*})
            }
        }

    }};

    (@repr required $value:ty) => { $value };
    (@repr $value:ty) => { Option<$value> };

    (@finalize required $self:ident.$key:ident) => {
        $self.$key.with_context(
            || format!("Required key {} not found",
            // TODO: field names instead of rust names here
            stringify!($key))
        )?
    };

    (@finalize $self:ident.$key:ident) => { $self.$key };
}

    // if we ever decide to make this a common utility
    // implement a way to have exhaustive enums
    #[macro_export]
    macro_rules! strenum {
    (
        $(#[$($attributes:tt)*])?
        $vis:vis $name:ident {
            $($field:ident $(= $value:literal)?,)*
        }
    ) => {
        $(#[$($attributes)*])?
        $vis enum $name {
            $($field,)*
            Other(String)
        }

        impl std::str::FromStr for $name {
            type Err = std::convert::Infallible;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(match s {
                    $(
                        $crate::strenum!(@field_string $field $($value)?)
                        => Self::$field,
                    )*
                    other => Self::Other(other.to_string()),
                })
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $(Self::$field => f.write_str(
                        $crate::strenum!(@field_string $field $($value)?)
                    ),)*
                    Self::Other(other) => f.write_str(&other),
                }
            }
        }
    };

    (@field_string $field:ident $value:literal) => { $value };
    (@field_string $field:ident) => { paste::paste! { stringify!([<$field:lower>]) } };
}
}

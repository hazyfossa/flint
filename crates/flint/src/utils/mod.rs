pub mod tty;
// pub mod plymouth;

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

        Ok(serde_json::from_slice(&buf)?)
    }
}

pub mod macros {
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
}

pub mod warn {
    use std::fmt::Debug;

    pub trait WarnExt<T> {
        fn warn(self) -> Option<T>;
    }

    impl<T, E: Debug> WarnExt<T> for std::result::Result<T, E> {
        fn warn(self) -> Option<T> {
            match self {
                Ok(value) => Some(value),
                Err(e) => {
                    tracing::warn!("{e:?}");
                    None
                }
            }
        }
    }
}

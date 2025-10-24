use std::{env, ffi::OsString, ops::Deref};

use anyhow::{Context, Result, anyhow};
use im::HashMap;
use tokio::process::Command;

pub mod prelude {
    pub use super::{Env, EnvParser, EnvVar};
    pub use crate::{define_env, env_parser_auto, env_parser_raw, utils::misc::OsStringExt};
}

// TODO: environment V2
// preserve type information as long as possible
// represent as hashmap entry only on import / export
// all other times - as a proper type
// + safe accessors
pub trait EnvVar: EnvParser {
    const KEY: &str;
}

pub trait EnvParser: Sized {
    fn serialize(&self) -> OsString;
    fn deserialize(value: OsString) -> Result<Self>;
}

#[macro_export]
macro_rules! env_parser_auto {
    ($target:ident) => {
        impl EnvParser for $target {
            #[inline]
            fn serialize(&self) -> std::ffi::OsString {
                self.0.to_string().into()
            }

            #[inline]
            fn deserialize(value: std::ffi::OsString) -> anyhow::Result<Self> {
                Ok(Self(value.try_to_string()?.parse()?))
            }
        }
    };
}

#[macro_export]
macro_rules! env_parser_raw {
    ($target:ident) => {
        impl EnvParser for $target {
            #[inline]
            fn serialize(&self) -> std::ffi::OsString {
                self.0.clone().into()
            }

            #[inline]
            fn deserialize(value: std::ffi::OsString) -> anyhow::Result<Self> {
                Ok(Self(value.into()))
            }
        }
    };
}

#[macro_export]
macro_rules! define_env {
    ($key:expr, $vis:vis $struct_name:ident($inner:ty)) => {
        #[derive(shrinkwraprs::Shrinkwrap)]
        $vis struct $struct_name($inner);
        impl EnvVar for $struct_name {
            const KEY: &str = $key;
        }
    };
}

#[macro_export]
macro_rules! impl_env {
    ($key:expr, $target:ident) => {
        paste::paste! {
        impl EnvVar for $target {
            const KEY: &str = $key;
        }}
    };
}

// This is a purely marker-abstraction
// as the value pulled from env would always be owned due to deserialization
//
// It exists to enforce the common-sense guarantee that for each Env
// there will only ever be one EnvVar<KEY> per unique KEY
pub struct PeekEnv<T>(T);

// This allows to look at the inner contents of a variable
// without allowing to pass it where a unique instance is needed
//
// Consider an example of VtNumber(u8)
//
impl<Inner, T: Deref<Target = Inner>> Deref for PeekEnv<T> {
    type Target = Inner;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

#[derive(Clone)]
pub struct Env {
    state: HashMap<String, OsString>,
}

pub fn current() -> Env {
    Env::from_values(
        env::vars_os()
            // Note: ignore all variables with non-unicode keys
            .filter_map(|(k, v)| Some((k.into_string().ok()?, v))),
    )
}

impl Env {
    pub fn from_values(values: impl IntoIterator<Item = (String, OsString)>) -> Self {
        Self {
            state: values.into_iter().collect(),
        }
    }

    pub fn peek<E: EnvVar>(&self) -> Result<PeekEnv<E>> {
        self.state
            .get(E::KEY)
            .ok_or(anyhow!("Environment variable {} is unset", E::KEY))
            .and_then(|value| Ok(PeekEnv(E::deserialize(value.clone())?)))
    }

    pub fn pull<E: EnvVar>(&mut self) -> Result<E> {
        let (value, state) = self
            .state
            .extract(E::KEY)
            .ok_or(anyhow!("Variable {} does not exist", E::KEY))?;

        self.state = state;

        E::deserialize(value).context(format!(
            "Variable {} exists, but contents are invalid",
            E::KEY
        ))
    }

    fn update_untyped(self, k: String, v: OsString) -> Self {
        Self {
            state: self.state.update(k, v),
        }
    }

    pub fn set<E: EnvVar>(self, var: E) -> Self {
        Self {
            state: self.state.update(E::KEY.to_string(), var.serialize()),
        }
    }

    pub fn merge<E: EnvContainer>(self, container: E) -> Self {
        container.apply_as_container(self)
    }

    pub fn merge_from<E: EnvContainerPartial>(self, container: &E) -> Self {
        container.apply_as_container(self)
    }

    pub fn to_vec(&self) -> Vec<OsString> {
        self.state
            .iter()
            .map(|pair| {
                let mut merged = OsString::new();

                merged.push(pair.0);
                merged.push("=");
                merged.push(pair.1);

                merged
            })
            .collect()
    }
}

pub trait EnvRecipient {
    fn set_env(&mut self, env: Env) -> Result<()>;
}

impl EnvRecipient for Command {
    fn set_env(&mut self, env: Env) -> Result<()> {
        self.env_clear().envs(env.state);
        Ok(())
    }
}

pub trait EnvContainer {
    fn apply_as_container(self, env: Env) -> Env;
}

pub trait EnvContainerPartial {
    fn apply_as_container(&self, env: Env) -> Env;
}

impl<T: EnvVar> EnvContainer for T {
    fn apply_as_container(self, env: Env) -> Env {
        env.set(self)
    }
}

impl EnvContainer for Env {
    fn apply_as_container(self, env: Env) -> Env {
        Self {
            state: env.state.union(self.state),
        }
    }
}

// NOTE: this is for untyped variables
// you would usually prefer typed ones instead
impl EnvContainer for &'static str {
    fn apply_as_container(self, env: Env) -> Env {
        let parts: Vec<&str> = self.split("=").collect();
        if parts.len() != 2 {
            panic!("Invalid environment update: {self}");
        }

        let (key, value) = (parts[0], parts[1]);
        env.update_untyped(key.to_string(), value.into())
    }
}

#[rustfmt::skip]
mod env_container_variadics {
    use super::*;

    macro_rules! var_impl {
        ( $( $name:ident )+ ) => {
            #[allow(non_camel_case_types)]
            impl<$($name: EnvContainer),+> EnvContainer for ($($name,)+)
            {
                fn apply_as_container(self, env: Env) -> Env {
                    let ($($name,)+) = self;
                    $(let env = env.merge($name);)+
                    env
                }
            }
        };
    }

    var_impl!           { a b }
    var_impl!          { a b c }
    var_impl!         { a b c d }
    var_impl!        { a b c d e }
    var_impl!       { a b c d e f }
    var_impl!      { a b c d e f g }
    var_impl!     { a b c d e f g h }
    var_impl!    { a b c d e f g h i }
    var_impl!   { a b c d e f g h i j }
    var_impl!  { a b c d e f g h i j k }
    var_impl! { a b c d e f g h i j k l }
}

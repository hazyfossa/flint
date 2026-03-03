use std::{
    collections::{HashMap, hash_map},
    ffi::OsString,
    path::PathBuf,
};

use anyhow::{Result, anyhow};
use serde::Deserialize;

pub trait EnvironmentParse<Repr>: Sized {
    fn env_serialize(self) -> Repr;
    fn env_deserialize(raw: Repr) -> Result<Self>;
}

impl<T: EnvironmentParse<String>> EnvironmentParse<OsString> for T {
    fn env_serialize(self) -> OsString {
        self.env_serialize().into()
    }

    fn env_deserialize(raw: OsString) -> Result<Self> {
        let value = raw
            .into_string()
            .map_err(|_| anyhow!("Variable contains invalid encoding"))?;

        Self::env_deserialize(value)
    }
}

macro_rules! env_parse_raw {
    ($ty:ty, $t:ident) => {
        impl EnvironmentParse<$ty> for $t {
            fn env_serialize(self) -> $ty {
                self.into()
            }

            fn env_deserialize(raw: $ty) -> Result<Self> {
                Ok(Self::from(raw))
            }
        }
    };
}

env_parse_raw!(OsString, PathBuf);
env_parse_raw!(OsString, OsString);
env_parse_raw!(String, String);

pub trait EnvironmentVariable: EnvironmentParse<OsString> {
    const KEY: &str;
}

// NOTE: this is for untyped variables
// you would usually prefer typed ones instead

pub use crate::_define_env as define_env;
#[macro_export]
macro_rules! _define_env {
    ($vis:vis $name:ident ($repr:ty) = parse $key:expr) => {
        impl crate::frame::environment::EnvironmentParse<std::ffi::OsString> for $name {
            fn env_serialize(self) -> std::ffi::OsString { self.0.env_serialize() }

            fn env_deserialize(raw: std::ffi::OsString) -> anyhow::Result<Self> {
                Ok(Self(<$repr>::env_deserialize(raw)?))
            }
        }

        crate::_define_env!($vis $name ($repr) = $key);
    };

    ($vis:vis $name:ident ($repr:ty) = $key:expr) => {
        #[derive(shrinkwraprs::Shrinkwrap)]
        $vis struct $name($repr);

        impl crate::frame::environment::EnvironmentVariable for $name {
            const KEY: &str = $key;
        }
    };
}

#[derive(Deserialize)]
pub struct Env(HashMap<String, OsString>);

impl Env {
    pub fn get<T: EnvironmentVariable>(&self) -> Result<T> {
        let raw = self
            .0
            .get(T::KEY)
            .ok_or(anyhow!("Variable {} does not exist", T::KEY))?;

        // TODO: zerocopy
        T::env_deserialize(raw.clone())
    }

    pub fn set<T: EnvDiff>(&mut self, e: T) {
        self.0.extend(e.to_env_diff());
    }

    pub fn from_values(values: impl IntoIterator<Item = (String, OsString)>) -> Self {
        Self(values.into_iter().collect())
    }

    pub fn to_vec(self) -> Vec<OsString> {
        self.0
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

impl IntoIterator for Env {
    type Item = (String, OsString);
    type IntoIter = hash_map::IntoIter<String, OsString>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

pub trait EnvDiff {
    fn to_env_diff(self) -> impl IntoIterator<Item = (String, OsString)>;
}

impl<T: EnvironmentVariable> EnvDiff for T {
    fn to_env_diff(self) -> impl IntoIterator<Item = (String, OsString)> {
        [(Self::KEY.to_string(), self.env_serialize())]
    }
}

// NOTE: this is for untyped variables
// you would usually prefer typed ones instead
impl EnvDiff for &'static str {
    fn to_env_diff(self) -> impl IntoIterator<Item = (String, OsString)> {
        let parts: Vec<&str> = self.split("=").collect();
        if parts.len() != 2 {
            panic!("Invalid environment update: {self}");
        }

        [(parts[0].into(), parts[1].into())]
    }
}

impl EnvDiff for Env {
    fn to_env_diff(self) -> impl IntoIterator<Item = (String, OsString)> {
        self
    }
}

#[rustfmt::skip]
mod env_container_variadics {
    use super::*;

    macro_rules! var_impl {
        ( $( $name:ident )+ ) => {
            #[allow(non_camel_case_types)]
            impl<$($name: EnvDiff),+> EnvDiff for ($($name,)+)
            {
                fn to_env_diff(self) -> impl IntoIterator<Item = (String, OsString)> {
                    let iter = std::iter::empty();
                    let ($($name,)+) = self;
                    $(let iter = iter.chain($name.to_env_diff());)+
                    iter
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

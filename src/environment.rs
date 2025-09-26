use std::{collections::HashMap, env, ffi::OsString, ops::Add, process::Command};

use anyhow::{Context, Result, anyhow};

pub trait EnvValue: Sized {
    const KEY: &str;

    fn serialize(self) -> OsString;
    fn deserialize(value: OsString) -> Result<Self>;

    fn current() -> Result<Self> {
        Self::deserialize(
            env::var_os(Self::KEY)
                .ok_or(anyhow!("Variable {} does not exist", Self::KEY))?
                .clone(),
        )
        .context("Variable exists, but contents are invalid")
    }
}

#[macro_export]
macro_rules! define_env {
    ($key:expr, $vis:vis $struct_name:ident($inner:ty)) => {
        #[derive(Debug, Clone)] // TODO: custom debug, ponder on clone
        $vis struct $struct_name($inner);

        impl $crate::environment::EnvValue for $struct_name {
            const KEY: &str = $key;

            fn serialize(self) -> std::ffi::OsString {
                self.0.to_string().into()
            }

            fn deserialize(value: std::ffi::OsString) -> anyhow::Result<Self> {
                use $crate::utils::misc::OsStringExt;
                Ok(Self(value.try_to_string()?.parse()?))
            }
        }

        impl std::ops::Deref for $struct_name {
            type Target = $inner;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
    };
}

#[derive(Debug, Clone)]
pub struct Env {
    store_set: HashMap<&'static str, OsString>,
    store_unset: Vec<&'static str>,
}

impl Add for Env {
    type Output = Env;

    fn add(mut self, mut other: Self) -> Self::Output {
        self.store_set.extend(other.store_set);
        self.store_unset.append(&mut other.store_unset);
        self
    }
}

impl Env {
    pub fn new() -> Self {
        Self {
            store_set: HashMap::new(),
            store_unset: Vec::new(),
        }
    }

    pub fn set<E: EnvValue>(mut self, var: E) -> Self {
        self.store_set.insert(E::KEY, var.serialize());
        self
    }

    pub fn unset<E: EnvValue>(mut self) -> Self {
        self.store_unset.push(E::KEY);
        self
    }
}

// TODO: derive with macros
pub trait EnvContainer {
    fn env_diff(self) -> Env;
}

pub trait EnvRecipient {
    fn merge_env(&mut self, ctx: Env) -> &mut Self;
}

// NOTE: dbus
// pub trait ExternalEnvRecipient {
//     fn merge_env(&self, ctx: EnvDiff) -> Result<()>;
// }

impl EnvRecipient for Command {
    fn merge_env(&mut self, ctx: Env) -> &mut Self {
        self.envs(ctx.store_set);

        for key in ctx.store_unset {
            self.env_remove(key);
        }

        self
    }
}

use std::{collections::HashMap, env, ffi::OsString, process::Command};

use anyhow::{Context, Result, anyhow};

pub trait EnvValue: Sized {
    const KEY: &str;

    // TODO: Consider making this own self
    fn serialize(&self) -> OsString;
    fn deserialize(value: OsString) -> Result<Self>;

    fn pull_from(ctx: &mut EnvContext) -> Result<Self> {
        let entry = Self::deserialize(
            ctx.0
                .get(Self::KEY)
                .ok_or(anyhow!("Variable {} does not exist", Self::KEY))?
                .clone(),
        )
        .context("Variable exists, but contents are invalid")?;

        ctx.0.remove(Self::KEY);
        Ok(entry)
    }

    #[inline]
    fn key(&self) -> String {
        Self::KEY.to_string()
    }

    #[inline]
    fn value(&self) -> OsString {
        self.serialize()
    }
}

#[macro_export]
macro_rules! env_impl {
    () => {
        fn serialize(&self) -> std::ffi::OsString {
            self.0.to_string().into()
        }

        fn deserialize(value: std::ffi::OsString) -> anyhow::Result<Self> {
            use $crate::utils::misc::OsStringExt; // TODO: is this fine?
            Ok(Self(value.try_to_string()?.parse()?))
        }
    };
}

pub trait EnvBundle {
    fn apply(self, ctx: &mut EnvContext);
}

// TODO: remove HashMap to reduce memory usage
// instead, define an EnvDiff (set + unset)
#[derive(Debug, Clone)]
pub struct EnvContext(HashMap<String, OsString>);

impl EnvContext {
    pub fn new(entries: impl Iterator<Item = impl EnvValue>) -> Self {
        Self(HashMap::from_iter(
            entries.map(|var| (var.key(), var.value())),
        ))
    }

    pub fn empty() -> Self {
        Self(HashMap::new())
    }

    pub fn current() -> Self {
        // TODO: avoid memory copy
        Self(
            env::vars_os()
                // Note: ignore all variables with non-unicode keys
                .filter_map(|(k, v)| Some((k.into_string().ok()?, v)))
                .collect(),
        )
    }

    pub fn set(&mut self, var: impl EnvValue) -> &mut Self {
        self.0.insert(var.key(), var.value());
        self
    }

    pub fn unset<EnvDefinition: EnvValue>(&mut self) -> &mut Self {
        self.0.remove(EnvDefinition::KEY);
        self
    }

    pub fn apply_bundle(&mut self, bundle: impl EnvBundle) -> &mut Self {
        bundle.apply(self);
        self
    }

    pub fn extend_with(&mut self, other: EnvContext) -> &mut Self {
        self.0.extend(other.0);
        self
    }
}

pub trait CommandEnvContextExt {
    fn with_env_context(&mut self, ctx: EnvContext) -> &mut Self;
}

impl CommandEnvContextExt for Command {
    fn with_env_context(&mut self, ctx: EnvContext) -> &mut Self {
        self.env_clear().envs(ctx.0)
    }
}

use std::{env, ffi::OsString, ops::Deref, process::Command};

use anyhow::{Context, Result, anyhow};

// TODO: benchmark
use im::{HashMap, hashmap::Entry};

pub trait EnvValue: Sized {
    const KEY: &str;

    fn serialize(self) -> OsString;
    fn deserialize(value: OsString) -> Result<Self>;
}

#[macro_export]
macro_rules! define_env {
    ($key:expr, $vis:vis $struct_name:ident($inner:ty)) => {
        #[derive(Debug, Clone)] // TODO: custom debug, ponder on clone
        $vis struct $struct_name($inner);

        impl $crate::environment::EnvValue for $struct_name {
            const KEY: &str = $key;

            #[inline]
            fn serialize(self) -> std::ffi::OsString {
                self.0.to_string().into()
            }

            #[inline]
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

// This is a purely marker-abstraction
// As the value pulled from env would always be owned due to deserialization
//
// Other designs could allow this to be optimized to a ref
pub struct PeekEnv<T>(T);

impl<T> Deref for PeekEnv<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
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
    pub fn empty() -> Self {
        Self {
            state: HashMap::new(),
        }
    }

    pub fn from_values(values: impl IntoIterator<Item = (String, OsString)>) -> Self {
        Self {
            state: values.into_iter().collect(),
        }
    }

    pub fn peek<E: EnvValue>(&self) -> Result<Option<PeekEnv<E>>> {
        match self.state.get(E::KEY) {
            None => Ok(None),
            Some(value) => Ok(Some(PeekEnv(E::deserialize(value.clone())?))),
        }
    }

    pub fn pull<E: EnvValue>(&mut self) -> Result<E> {
        let (value, state) = self
            .state
            .extract(E::KEY)
            .ok_or(anyhow!("Variable {} does not exist", E::KEY))?;

        self.state = state;

        E::deserialize(value).context("Variable exists, but contents are invalid")
    }

    // This is an internal method
    // Callers should use env.set() to set variables
    // In the simplest case of one variable, set == bind
    // For N variables set == N binds
    fn bind<E: EnvValue>(self, var: E) -> Self {
        Self {
            state: self.state.update(E::KEY.to_string(), var.serialize()),
        }
    }

    pub fn set<E: EnvContainer>(self, container: E) -> Self {
        container.apply(self)
    }

    pub fn ensure<E: EnvValue + Default>(&mut self) -> Result<E> {
        let value = match self.state.entry(E::KEY.to_string()) {
            Entry::Occupied(entry) => entry.get().clone(),
            Entry::Vacant(entry) => entry.insert(E::default().serialize()).clone(),
        };

        E::deserialize(value)
    }
}

pub trait EnvRecipient {
    fn set_env(&mut self, ctx: Env) -> &mut Self;
}

impl EnvRecipient for Command {
    fn set_env(&mut self, ctx: Env) -> &mut Self {
        self.env_clear().envs(ctx.state);
        self
    }
}

pub trait EnvContainer {
    fn apply(self, env: Env) -> Env;
}

macro_rules! variadic_env_impl {
    ( $( $name:ident )+ ) => {
        #[allow(non_camel_case_types)]
        impl<$($name: EnvValue),+> EnvContainer for ($($name,)+)
        {
            fn apply(self, env: Env) -> Env {
                let ($($name,)+) = self;
                $(let env = env.bind($name);)+
                env
            }
        }
    };
}

variadic_env_impl! { a b }
variadic_env_impl! { a b c }
variadic_env_impl! { a b c d }
variadic_env_impl! { a b c d e }
variadic_env_impl! { a b c d e f }
variadic_env_impl! { a b c d e f g }
variadic_env_impl! { a b c d e f g h }
variadic_env_impl! { a b c d e f g h i }
variadic_env_impl! { a b c d e f g h i j }
variadic_env_impl! { a b c d e f g h i j k }
variadic_env_impl! { a b c d e f g h i j k l }

impl<T: EnvValue> EnvContainer for T {
    fn apply(self, env: Env) -> Env {
        env.bind(self)
    }
}

impl EnvContainer for Env {
    fn apply(self, env: Env) -> Env {
        Self {
            state: env.state.union(self.state),
        }
    }
}

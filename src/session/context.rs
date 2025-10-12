use anyhow::Result;

use crate::environment::{Env, EnvContainer};

crate::define_env!("XDG_VTNR", pub VtNumber(u16));

impl From<u16> for VtNumber {
    fn from(value: u16) -> Self {
        Self(value)
    }
}

crate::define_env!("XDG_SEAT", pub Seat(String));

impl Default for Seat {
    fn default() -> Self {
        // man sd-login says that seat0 always exists
        Self("seat0".into())
    }
}

pub struct SessionContext {
    pub vt: VtNumber,
    pub seat: Seat,

    pub env: Env,
}

impl SessionContext {
    pub fn from_env(mut env: Env) -> Result<Self> {
        Ok(Self {
            vt: env.pull()?,
            seat: env.ensure()?,
            env,
        })
    }
}

impl EnvContainer for SessionContext {
    fn apply(self, env: Env) -> Env {
        env.merge(self.env)
    }
}

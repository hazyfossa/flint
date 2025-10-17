use anyhow::Result;
use shrinkwraprs::Shrinkwrap;

use crate::environment::{EnvContainer, prelude::*};

#[derive(Shrinkwrap, Clone)]
pub struct VtNumber(u16);

impl_env!("XDG_VTNR", VtNumber);
env_parser_auto!(VtNumber);

impl From<u16> for VtNumber {
    fn from(value: u16) -> Self {
        Self(value)
    }
}

define_env!("XDG_SEAT", pub Seat(String));
env_parser_auto!(Seat);

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
            seat: env.pull().unwrap_or_default(), // Propagate if seat exists but invalid
            env,
        })
    }
}

impl EnvContainer for SessionContext {
    fn apply(self, env: Env) -> Env {
        env.merge(self.env)
    }
}

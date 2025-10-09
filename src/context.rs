use anyhow::Result;

use crate::{environment::Env, tty::VtNumber};

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

    pub inherit_env: Env,
}

impl SessionContext {
    pub fn from_env(mut env: Env) -> Result<Self> {
        Ok(Self {
            vt: env.pull()?,
            seat: env.ensure()?,
            inherit_env: env,
        })
    }
}

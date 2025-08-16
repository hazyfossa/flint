mod keyboard;

use std::{fmt::Display, os::fd::OwnedFd};

use crate::EnvValue;

pub struct VtNumber(String);

impl Display for VtNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad(&self.0)
    }
}

impl EnvValue for VtNumber {
    const KEY: &str = "XDG_VTNR";
    crate::env_impl!();
}

pub struct VT {
    descriptor: OwnedFd,
}

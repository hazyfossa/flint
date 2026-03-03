use anyhow::Result;

use crate::{
    bind::tty::VtNumber,
    frame::environment::{EnvironmentParse, EnvironmentVariable, define_env},
};

impl EnvironmentVariable for VtNumber {
    const KEY: &str = "XDG_VTNR";
}

impl EnvironmentParse<String> for VtNumber {
    fn env_serialize(self) -> String {
        self.0.to_string()
    }

    fn env_deserialize(raw: String) -> Result<Self> {
        Ok(Self(raw.parse()?))
    }
}

define_env!(pub Seat(String) = parse "XDG_SEAT");

impl Seat {
    pub fn into_id(self) -> String {
        self.0
    }
}

impl Default for Seat {
    fn default() -> Self {
        // man sd-login says that seat0 always exists
        Self("seat0".into())
    }
}

// UserIncomplete, Manager, Background and None are not here as those aren't relevant
#[allow(dead_code)]
pub enum SessionClass {
    User { early: bool, light: bool },
    Greeter,
    LockScreen,
}

impl EnvironmentVariable for SessionClass {
    const KEY: &str = "XDG_SESSION_CLASS";
}

impl EnvironmentParse<String> for SessionClass {
    fn env_serialize(self) -> String {
        match self {
            Self::User { early, light } => {
                let mut string = "user".to_string();
                if early {
                    string += "-early"
                }
                if light {
                    string += "-light"
                }
                string
            }
            Self::Greeter => "greeter".to_string(),
            Self::LockScreen => "lock-screen".to_string(),
        }
    }

    fn env_deserialize(_value: String) -> Result<Self> {
        todo!()
    }
}

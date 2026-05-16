use anyhow::Result;
use envy::{define_env, parse::EnvironmentParse};

define_env!(
    #[derive(PartialEq)]
    pub VtNumber(u16) = "XDG_VTNR"
);

impl VtNumber {
    pub fn to_tty_string(&self) -> String {
        format!("tty{}", self.to_string())
    }
}

define_env!(pub Seat(String) = "XDG_SEAT");

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

define_env!(SessionClass = #custom "XDG_SESSION_CLASS");

impl EnvironmentParse<String> for SessionClass {
    type Error = std::convert::Infallible;

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

    fn env_deserialize(_value: String) -> Result<Self, Self::Error> {
        todo!()
    }
}

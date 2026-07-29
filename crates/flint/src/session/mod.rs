use envy::{define_env, parse::EnvironmentParse};

mod xdg;

// UserIncomplete, Manager, Background and None are not here as those aren't relevant
#[allow(dead_code)]
pub enum SessionClass {
    User { early: bool, light: bool },
    Greeter,
    LockScreen,
}

impl SessionClass {
    fn user_default() -> Self {
        Self::User {
            early: false,
            light: false,
        }
    }
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

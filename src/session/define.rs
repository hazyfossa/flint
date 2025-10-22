use anyhow::Result;
use serde::de::DeserializeOwned;

use std::path::PathBuf;

use super::metadata::SessionMetadataLookup;
use crate::{
    environment::{EnvContainerPartial, prelude::*},
    login::VtRenderMode,
    session::manager::{SessionContext, SessionManager},
};

pub mod prelude {
    pub use crate::{
        login::context::VtNumber,
        session::{
            define,
            manager::SessionContext,
            metadata::{self, SessionMap, SessionMetadata},
        },
    };
    pub use serde::Deserialize;
}

pub trait SessionType: Sized + SessionMetadataLookup {
    const XDG_TYPE: &str;

    type ManagerConfig: Default + DeserializeOwned;

    const VT_RENDER_MODE: VtRenderMode = VtRenderMode::Graphics;

    fn setup_session(
        config: &Self::ManagerConfig,
        context: &mut SessionContext,
        executable: PathBuf,
    ) -> Result<()>;
}

define_env!("XDG_SESSION_TYPE", pub SessionTypeEnv(String));
env_parser_auto!(SessionTypeEnv);

impl<T: SessionType> EnvContainerPartial for SessionManager<T> {
    fn apply_as_container(&self, env: Env) -> Env {
        env.set(SessionTypeEnv(T::XDG_TYPE.to_string()))
    }
}

#[macro_export]
macro_rules! session_type {
    ($session_type:tt) => {
        crate::session::$session_type::Session
    };
}

#[macro_export]
macro_rules! sessions {
    ([$($session:tt),+]) => { // fn sessions([*session_types])
        $( pub mod $session; )+

        $crate::scope!{($all:tt) => {
            #[macro_export]
            macro_rules! _dispatch_session {
                ($xdg_type:expr => $function:ident($all($args:tt)*)) => { // string => function(*arguments)
                    match $xdg_type {
                        // for T in session_types:
                        //     T::XDG_TYPE => function::<T>(*arguments)
                        $( <session_type!($session)>::XDG_TYPE => $function::<session_type!($session)>($all($args)*), )+
                        //
                        other => anyhow::bail!("{other} is not a valid session type."),
                    }
                }
            }
            pub use _dispatch_session as dispatch_session; // return _dispatch_session
        }}
    }
}

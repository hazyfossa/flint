use std::path::Path;

use anyhow::Result;
use facet::Facet;

use super::{manager, metadata};
use crate::{login::VtRenderMode, trait_alias};

pub mod prelude {
    pub use super::{SessionType, SessionTypeTag};
    pub use crate::session::{manager::SessionContext, metadata::FreedesktopMetadata};
}

trait_alias!(Config = for<'f> Facet<'f> + Default);

pub trait SessionType: metadata::SessionMetadataLookup + Config {
    // This should equal xdg type if possible
    const TAG: &SessionTypeTag<str>;

    const VT_RENDER_MODE: VtRenderMode = VtRenderMode::Graphics;

    async fn setup_session(
        &self,
        context: &mut manager::SessionContext,
        executable: &Path,
    ) -> Result<()>;
}

pub type SessionTypeTag<T: AsRef<str> = String> = T;

pub use crate::_sessions as sessions;

#[macro_export]
macro_rules! _sessions {
    ([$($session:ident),+]) => { paste::paste! {
        $( pub mod $session; )+
        $( use $session::SessionManager as [<$session T>]; )+

        #[derive(facet::Facet, Default, Clone)]
        pub struct SessionTypeConfig<T> {
            #[facet(flatten)]
            pub config: T,
            #[facet(rename = "session")]
            pub entries: metadata::SessionMetadataMap<T>,
        }

        #[derive(facet::Facet, Default)]
        pub struct Config {
            $( $session:
                Option<
                  SessionTypeConfig < [<$session T>] >
                >,
            )+
        }

        // impl SessionConfig {
        //     fn get<T: define::SessionType>(&self) ->  {
        //         match T::TAG {
        //             $([<$session Config>]::TAG => self.$session.unwrap_or_default()),+
        //         }
        //     }
        // }

        $crate::scope!{($all:tt) => {
            #[macro_export]
            macro_rules! dispatch_session {
                // string => function(*arguments)
                ($tag:expr => $function:ident($all($args:tt)*) $all($post:tt)?) => {
                    match $tag {
                        // T::tag => function::<T>(*arguments)
                        $( <[<$session Session>]>::tag =>
                        $function::<[<$session Session>]>($all($args)*) $all($post)?, )+
                        //
                        // TODO "all"
                        other => anyhow::bail!("{other} is not a valid session type."),
                    }
                }
            }
        }}
    }}
}

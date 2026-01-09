pub use crate::_sessions as sessions;
#[macro_export]
macro_rules! _sessions {
    ([$($session:ident),+]) => { paste::paste! {
        $( pub mod [<$session:lower>]; )+
        $( use [<$session:lower>]::SessionConfig as [<$session Tag>]; )+

        const ALL_TAGS: &[&str] = &[$(stringify!([<$session:lower>]),)+];



        // #[repr(u8)]
        // #[derive(facet::Facet)]
        #[enum_dispatch::enum_dispatch(SessionType)]
        pub enum SessionInner {
            $([<$session Tag>],)+
        }

        // I don't know why enum_dispatch doesn't recursively do that already
        impl crate::session::metadata::SessionMetadataLookup for SessionInner {
            fn lookup_metadata(&self, id: &str) -> Result<metadata::SessionDefinition> {
                match self {
                    $(Self::[<$session Tag>](i) => i.lookup_metadata(id),)+
                }
            }

            fn lookup_metadata_all(&self) -> metadata::SessionMetadataMap {
                match self {
                    $(Self::[<$session Tag>](i) => i.lookup_metadata_all(),)+
                }
            }
        }

        impl SessionInner {
            fn parse(tag: &SessionTypeTag, value: facet_value::Value) -> anyhow::Result<Self> {
                Ok(match tag.as_ref() {
                    $(stringify!([<$session:lower>]) => Self::[<$session Tag>](
                        facet_value::from_value(value)?
                    ),)+
                    other => anyhow::bail!("{other} is not a supported session type")
                })

            }
        }
    }}
}

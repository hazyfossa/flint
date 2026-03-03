pub mod metadata;

use anyhow::Result;
use facet::Facet;

use crate::{core::SessionContext, session::metadata::SessionMetadataMap};

pub mod prelude {
    pub use super::{SessionType, metadata::FreedesktopMetadata};
    pub use crate::core::SessionContext;
}

pub trait SessionType: metadata::SessionMetadataLookup {
    async fn setup_session(&self, context: &mut SessionContext) -> Result<()>;
}

#[derive(Facet)]
pub struct ConfigCell<T> {
    #[facet(flatten)]
    manager_config: T,
    #[facet(rename = "entry")]
    entries: SessionMetadataMap<T>,
}

crate::plug_mod!(
    (trait: SessionType, config_cell: ConfigCell, name: SessionManager)
    {
        pub x11 = "X11",
        pub wayland = "wayland",
        // pub tty = "tty",
    }
);

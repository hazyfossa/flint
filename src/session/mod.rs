pub mod manager;
pub mod metadata;

use std::path::Path;

use anyhow::Result;
use facet::Facet;

use crate::{bind::tty::VtRenderMode, session::metadata::SessionMetadataMap};

pub mod prelude {
    pub use super::SessionType;
    pub use crate::session::{manager::SessionContext, metadata::FreedesktopMetadata};
}

pub trait SessionType: metadata::SessionMetadataLookup {
    const VT_RENDER_MODE: VtRenderMode = VtRenderMode::Graphics;

    async fn setup_session(
        &self,
        context: &mut manager::SessionContext,
        executable: &Path,
    ) -> Result<()>;
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
        pub tty = "tty",
    }
);

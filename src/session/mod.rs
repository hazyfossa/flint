pub mod manager;
pub mod metadata;

use std::path::Path;

use anyhow::Result;
use facet::Facet;

use crate::login::VtRenderMode;

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
struct CommonConfig {}

crate::plug_mod!((trait: SessionType, common: CommonConfig, name: SessionManager) {
    pub x11 = "X11",
    pub wayland = "wayland",
    pub tty = "tty",
});

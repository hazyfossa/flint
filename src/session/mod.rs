pub mod macros;
pub mod manager;
pub mod metadata;

use anyhow::Result;

use std::path::Path;

use crate::login::VtRenderMode;
use macros::sessions;

sessions!([X11, Wayland, TTY]);

#[async_trait::async_trait]
#[enum_dispatch::enum_dispatch]
pub trait SessionType: metadata::SessionMetadataLookup {
    // This should equal XDG_TYPE if possible
    fn tag(&self) -> &'static str;

    fn vt_render_mode(&self) -> VtRenderMode {
        VtRenderMode::Graphics
    }

    async fn setup_session(
        &self,
        context: &mut manager::SessionContext,
        executable: &Path,
    ) -> Result<()>;
}

pub type SessionTypeTag<T: AsRef<str> = String> = T;

pub use manager::SessionManager;

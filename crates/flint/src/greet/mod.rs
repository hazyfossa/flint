use crate::metadata;
use eyre::Result;
use plug::plug;

// #[plug]
pub trait Greeter {
    // This should be merged with plug Init (sessions is common config)
    async fn start(sessions: metadata::ForGreeeter<'_>) -> Result<Box<Self>>;

    async fn display(&mut self, message: String) -> Result<()>;
}

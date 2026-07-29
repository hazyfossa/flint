use anyhow::Result;
use dyn_utils::dyn_trait;

#[dyn_trait]
pub trait Greeter {
    async fn start() -> Self;
    async fn display(&mut self, message: String) -> Result<()>;
}

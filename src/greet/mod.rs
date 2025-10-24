use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GreeterConfig {
    session: Option<String>,
    executable: PathBuf,
}

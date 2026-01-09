use std::path::PathBuf;

use facet::Facet;

#[derive(Facet)]
pub struct GreeterConfig {
    session: Option<String>,
    executable: PathBuf,
}

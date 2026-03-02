use std::path::PathBuf;

use facet::Facet;

#[derive(Facet)]
pub struct GreeterConfig {
    session: Option<String>, // TODO: tag types in plug?
    executable: PathBuf,
}

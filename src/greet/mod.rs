use std::path::PathBuf;

use facet::Facet;

use crate::session::define::SessionTypeTag;

#[derive(Facet)]
pub struct GreeterConfig {
    session: Option<SessionTypeTag>,
    executable: PathBuf,
}

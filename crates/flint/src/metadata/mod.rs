use serde::{Deserialize, Serialize};

mod xdg;

// An opaque session metadata identifier
// also known as a handle
pub type ID = u64;

#[derive(Serialize, Deserialize)]
enum Source {
    XDG,    // { path: PathBuf }
    Config, // { path: PathBuf }
    Special,
}

#[derive(Serialize, Deserialize)]
pub struct Summary {
    name: String,
    description: Option<String>,
    source: Source,
}

// enum Target {

// }

// enum Sp {
//     A,
//     B,
// }

// #[derive(Serialize, Deserialize)]
// enum WSp {
//     Sp(Sp),
//     #[serde(untagged)]
//     Other(String),
// }

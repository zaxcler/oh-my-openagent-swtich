mod fingerprint;
pub mod jsonc;
mod merge;

pub use fingerprint::fingerprint;
pub use merge::{build_oh_my_openagent, merge_opencode};

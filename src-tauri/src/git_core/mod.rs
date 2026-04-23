// Git core operations wrapper around git2-rs
//
// All git2 raw types are confined to this module.
// Only domain types (GitRepository, GitReference, ProgressSender, etc.) are exported.

pub mod repository;

pub use repository::{GitReference, GitRepository, ProgressSender};

// Error types for the Git GUI application

use serde::{Deserialize, Serialize};

/// Core error type for all Git operations.
/// Maps git2 errors and IO errors into domain-specific variants.
#[derive(Debug, thiserror::Error, Clone)]
pub enum GitError {
    #[error("Repository not found: {path}")]
    RepositoryNotFound { path: String },
    #[error("Not a git repository: {path}")]
    NotARepository { path: String },
    #[error("Merge conflict")]
    MergeConflict { files: Vec<String> },
    #[error("Authentication failed")]
    AuthenticationFailed,
    #[error("Remote rejected: {reason}")]
    RemoteRejected { reason: String },
    #[error("Network error: {message}")]
    NetworkError { message: String },
    #[error("git2 error: {0}")]
    Git2(String),
    #[error("IO error: {0}")]
    Io(String),
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
}

impl GitError {
    /// Returns a string identifier for each error variant.
    pub fn error_type_name(&self) -> &str {
        match self {
            GitError::RepositoryNotFound { .. } => "RepositoryNotFound",
            GitError::NotARepository { .. } => "NotARepository",
            GitError::MergeConflict { .. } => "MergeConflict",
            GitError::AuthenticationFailed => "AuthenticationFailed",
            GitError::RemoteRejected { .. } => "RemoteRejected",
            GitError::NetworkError { .. } => "NetworkError",
            GitError::Git2(_) => "Git2",
            GitError::Io(_) => "Io",
            GitError::InvalidArgument(_) => "InvalidArgument",
        }
    }
}

impl Serialize for GitError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("error_type", self.error_type_name())?;
        map.serialize_entry("message", &self.to_string())?;
        map.end()
    }
}

/// Structured error type for IPC communication between Rust backend and frontend.
/// All backend errors are converted to this type before being sent to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcError {
    pub error_type: String,
    pub message: String,
}

impl std::fmt::Display for IpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.error_type, self.message)
    }
}

impl From<git2::Error> for GitError {
    fn from(e: git2::Error) -> Self {
        match e.class() {
            git2::ErrorClass::Net => GitError::NetworkError {
                message: e.message().to_string(),
            },
            git2::ErrorClass::Ssh => GitError::AuthenticationFailed,
            git2::ErrorClass::Repository => {
                let msg = e.message().to_string();
                if msg.contains("not found") {
                    GitError::RepositoryNotFound { path: msg }
                } else {
                    GitError::Git2(msg)
                }
            }
            git2::ErrorClass::Index if e.message().contains("conflict") => {
                GitError::MergeConflict { files: vec![] }
            }
            _ => GitError::Git2(e.message().to_string()),
        }
    }
}

impl From<std::io::Error> for GitError {
    fn from(e: std::io::Error) -> Self {
        GitError::Io(e.to_string())
    }
}

impl From<GitError> for IpcError {
    fn from(e: GitError) -> Self {
        IpcError {
            error_type: e.error_type_name().to_string(),
            message: e.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// Generates an arbitrary `GitError` variant for property testing.
    fn arb_git_error() -> impl Strategy<Value = GitError> {
        prop_oneof![
            "\\PC+".prop_map(|path| GitError::RepositoryNotFound { path }),
            "\\PC+".prop_map(|path| GitError::NotARepository { path }),
            prop::collection::vec("\\PC+", 0..5)
                .prop_map(|files| GitError::MergeConflict { files }),
            Just(GitError::AuthenticationFailed),
            "\\PC+".prop_map(|reason| GitError::RemoteRejected { reason }),
            "\\PC+".prop_map(|message| GitError::NetworkError { message }),
            "\\PC+".prop_map(GitError::Git2),
            "\\PC+".prop_map(GitError::Io),
            "\\PC+".prop_map(GitError::InvalidArgument),
        ]
    }

    proptest! {
        /// **Validates: Requirements 16.5**
        ///
        /// Feature: rust-git-gui-client, Property 15: IPC 错误结构保持
        ///
        /// For any GitError variant, converting to IpcError SHALL produce
        /// a non-empty error_type and a non-empty message.
        #[test]
        fn prop_ipc_error_structure_preserved(error in arb_git_error()) {
            let ipc_error: IpcError = IpcError::from(error);
            prop_assert!(!ipc_error.error_type.is_empty(), "error_type must not be empty");
            prop_assert!(!ipc_error.message.is_empty(), "message must not be empty");
        }
    }
}

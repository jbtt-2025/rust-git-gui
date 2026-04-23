use std::cell::Cell;

/// Manages credential callbacks for git2 remote operations.
/// Supports SSH key auth (ssh-agent, default key paths) and HTTPS basic auth.
pub struct CredentialManager;

impl CredentialManager {
    pub fn new() -> Self {
        Self
    }

    /// Create git2 RemoteCallbacks with credential handling and optional progress reporting.
    pub fn create_callbacks<'a>(
        &self,
        progress: Option<crate::git_core::ProgressSender>,
    ) -> git2::RemoteCallbacks<'a> {
        let mut callbacks = git2::RemoteCallbacks::new();

        // Track attempts to avoid infinite credential loops
        let attempts = Cell::new(0u32);

        callbacks.credentials(move |url, username_from_url, allowed_types| {
            let attempt = attempts.get();
            if attempt > 3 {
                return Err(git2::Error::from_str("too many authentication attempts"));
            }
            attempts.set(attempt + 1);

            let username = username_from_url.unwrap_or("git");

            if allowed_types.contains(git2::CredentialType::SSH_KEY) {
                // Try ssh-agent first
                if let Ok(cred) = git2::Cred::ssh_key_from_agent(username) {
                    return Ok(cred);
                }

                // Try default key paths
                let home = dirs_fallback();
                for key_name in &["id_ed25519", "id_rsa"] {
                    let key_path = home.join(".ssh").join(key_name);
                    if key_path.exists() {
                        let pub_path = home.join(".ssh").join(format!("{}.pub", key_name));
                        let pub_key = if pub_path.exists() {
                            Some(pub_path.as_path())
                        } else {
                            None
                        };
                        if let Ok(cred) =
                            git2::Cred::ssh_key(username, pub_key, &key_path, None)
                        {
                            return Ok(cred);
                        }
                    }
                }

                Err(git2::Error::from_str(
                    "no SSH key found for authentication",
                ))
            } else if allowed_types.contains(git2::CredentialType::USER_PASS_PLAINTEXT) {
                // Try git credential helper
                git2::Cred::credential_helper(
                    &git2::Config::open_default().unwrap_or_else(|_| {
                        // Fallback: empty config
                        git2::Config::new().expect("failed to create empty config")
                    }),
                    url,
                    username_from_url,
                )
            } else if allowed_types.contains(git2::CredentialType::DEFAULT) {
                git2::Cred::default()
            } else {
                Err(git2::Error::from_str("unsupported credential type"))
            }
        });

        if let Some(progress) = progress {
            callbacks.transfer_progress(move |stats| {
                progress(
                    stats.received_objects(),
                    stats.total_objects(),
                    stats.received_bytes(),
                );
                true
            });
        }

        callbacks
    }
}

/// Get the user's home directory without pulling in extra crates.
fn dirs_fallback() -> std::path::PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        return std::path::PathBuf::from(home);
    }
    #[cfg(target_os = "windows")]
    if let Some(profile) = std::env::var_os("USERPROFILE") {
        return std::path::PathBuf::from(profile);
    }
    std::path::PathBuf::from(".")
}

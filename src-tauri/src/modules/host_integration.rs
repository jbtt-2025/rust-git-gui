// Host integration module for GitHub and GitLab platforms
//
// Provides URL generation for remote hosting platforms and stub
// implementations for API operations (authenticate, list/create PRs).

use crate::error::GitError;
use crate::models::{CreatePrParams, PullRequest};

// === Error Type ===

#[derive(Debug, Clone, thiserror::Error)]
pub enum HostError {
    #[error("Authentication failed: {reason}")]
    AuthFailed { reason: String },
    #[error("API not implemented: {message}")]
    NotImplemented { message: String },
    #[error("Invalid remote URL: {url}")]
    InvalidUrl { url: String },
}

impl From<HostError> for GitError {
    fn from(e: HostError) -> Self {
        match e {
            HostError::AuthFailed { .. } => GitError::AuthenticationFailed,
            HostError::NotImplemented { message } => GitError::InvalidArgument(message),
            HostError::InvalidUrl { url } => {
                GitError::InvalidArgument(format!("Invalid remote URL: {url}"))
            }
        }
    }
}

// === UserInfo ===

#[derive(Debug, Clone)]
pub struct UserInfo {
    pub username: String,
    pub display_name: Option<String>,
    pub email: Option<String>,
}

// === HostProvider Trait ===

pub trait HostProvider {
    fn authenticate(&mut self, token: &str) -> Result<UserInfo, HostError>;
    fn list_pull_requests(&self, repo_url: &str) -> Result<Vec<PullRequest>, HostError>;
    fn create_pull_request(&self, params: CreatePrParams) -> Result<PullRequest, HostError>;
    fn repo_web_url(&self, remote_url: &str) -> Option<String>;
    fn commit_web_url(&self, remote_url: &str, sha: &str) -> Option<String>;
    fn branch_web_url(&self, remote_url: &str, branch: &str) -> Option<String>;
}


// === URL Parsing Helper ===

/// Extracts (owner, repo) from a remote URL.
/// Supports both SSH (git@host:owner/repo.git) and HTTPS (https://host/owner/repo.git) formats.
fn parse_owner_repo(remote_url: &str) -> Option<(String, String)> {
    // SSH format: git@github.com:owner/repo.git
    if let Some(rest) = remote_url.strip_prefix("git@") {
        let colon_pos = rest.find(':')?;
        let path = &rest[colon_pos + 1..];
        return parse_path_segments(path);
    }

    // HTTPS format: https://github.com/owner/repo.git
    if remote_url.starts_with("https://") || remote_url.starts_with("http://") {
        let without_scheme = if let Some(rest) = remote_url.strip_prefix("https://") {
            rest
        } else {
            remote_url.strip_prefix("http://")?
        };
        // Skip the host part
        let slash_pos = without_scheme.find('/')?;
        let path = &without_scheme[slash_pos + 1..];
        return parse_path_segments(path);
    }

    None
}

/// Parses "owner/repo.git" or "owner/repo" into (owner, repo).
fn parse_path_segments(path: &str) -> Option<(String, String)> {
    let path = path.trim_end_matches('/');
    let parts: Vec<&str> = path.splitn(3, '/').collect();
    if parts.len() < 2 {
        return None;
    }
    let owner = parts[0];
    let repo = parts[1].trim_end_matches(".git");
    if owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some((owner.to_string(), repo.to_string()))
}

// === GitHub Provider ===

#[derive(Debug, Clone)]
pub struct GitHubProvider {
    token: Option<String>,
}

impl GitHubProvider {
    pub fn new() -> Self {
        Self { token: None }
    }
}

impl HostProvider for GitHubProvider {
    fn authenticate(&mut self, token: &str) -> Result<UserInfo, HostError> {
        // Stub: store token, return placeholder user
        self.token = Some(token.to_string());
        Err(HostError::NotImplemented {
            message: "GitHub authentication requires HTTP client (not yet integrated)".into(),
        })
    }

    fn list_pull_requests(&self, _repo_url: &str) -> Result<Vec<PullRequest>, HostError> {
        Err(HostError::NotImplemented {
            message: "GitHub list_pull_requests requires HTTP client (not yet integrated)".into(),
        })
    }

    fn create_pull_request(&self, _params: CreatePrParams) -> Result<PullRequest, HostError> {
        Err(HostError::NotImplemented {
            message: "GitHub create_pull_request requires HTTP client (not yet integrated)".into(),
        })
    }

    fn repo_web_url(&self, remote_url: &str) -> Option<String> {
        let (owner, repo) = parse_owner_repo(remote_url)?;
        Some(format!("https://github.com/{owner}/{repo}"))
    }

    fn commit_web_url(&self, remote_url: &str, sha: &str) -> Option<String> {
        let (owner, repo) = parse_owner_repo(remote_url)?;
        Some(format!("https://github.com/{owner}/{repo}/commit/{sha}"))
    }

    fn branch_web_url(&self, remote_url: &str, branch: &str) -> Option<String> {
        let (owner, repo) = parse_owner_repo(remote_url)?;
        Some(format!("https://github.com/{owner}/{repo}/tree/{branch}"))
    }
}

// === GitLab Provider ===

#[derive(Debug, Clone)]
pub struct GitLabProvider {
    token: Option<String>,
}

impl GitLabProvider {
    pub fn new() -> Self {
        Self { token: None }
    }
}

impl HostProvider for GitLabProvider {
    fn authenticate(&mut self, token: &str) -> Result<UserInfo, HostError> {
        self.token = Some(token.to_string());
        Err(HostError::NotImplemented {
            message: "GitLab authentication requires HTTP client (not yet integrated)".into(),
        })
    }

    fn list_pull_requests(&self, _repo_url: &str) -> Result<Vec<PullRequest>, HostError> {
        Err(HostError::NotImplemented {
            message: "GitLab list_pull_requests requires HTTP client (not yet integrated)".into(),
        })
    }

    fn create_pull_request(&self, _params: CreatePrParams) -> Result<PullRequest, HostError> {
        Err(HostError::NotImplemented {
            message: "GitLab create_pull_request requires HTTP client (not yet integrated)".into(),
        })
    }

    fn repo_web_url(&self, remote_url: &str) -> Option<String> {
        let (owner, repo) = parse_owner_repo(remote_url)?;
        Some(format!("https://gitlab.com/{owner}/{repo}"))
    }

    fn commit_web_url(&self, remote_url: &str, sha: &str) -> Option<String> {
        let (owner, repo) = parse_owner_repo(remote_url)?;
        Some(format!("https://gitlab.com/{owner}/{repo}/-/commit/{sha}"))
    }

    fn branch_web_url(&self, remote_url: &str, branch: &str) -> Option<String> {
        let (owner, repo) = parse_owner_repo(remote_url)?;
        Some(format!("https://gitlab.com/{owner}/{repo}/-/tree/{branch}"))
    }
}

// === HostIntegration (Facade) ===

pub struct HostIntegration {
    provider: Box<dyn HostProvider>,
}

impl HostIntegration {
    pub fn new(provider: Box<dyn HostProvider>) -> Self {
        Self { provider }
    }

    pub fn authenticate(&mut self, token: &str) -> Result<UserInfo, HostError> {
        self.provider.authenticate(token)
    }

    pub fn list_pull_requests(&self, repo_url: &str) -> Result<Vec<PullRequest>, HostError> {
        self.provider.list_pull_requests(repo_url)
    }

    pub fn create_pull_request(&self, params: CreatePrParams) -> Result<PullRequest, HostError> {
        self.provider.create_pull_request(params)
    }

    pub fn repo_web_url(&self, remote_url: &str) -> Option<String> {
        self.provider.repo_web_url(remote_url)
    }

    pub fn commit_web_url(&self, remote_url: &str, sha: &str) -> Option<String> {
        self.provider.commit_web_url(remote_url, sha)
    }

    pub fn branch_web_url(&self, remote_url: &str, branch: &str) -> Option<String> {
        self.provider.branch_web_url(remote_url, branch)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    // === parse_owner_repo tests ===

    #[test]
    fn test_parse_ssh_url() {
        let result = parse_owner_repo("git@github.com:user/repo.git");
        assert_eq!(result, Some(("user".into(), "repo".into())));
    }

    #[test]
    fn test_parse_ssh_url_no_git_suffix() {
        let result = parse_owner_repo("git@github.com:user/repo");
        assert_eq!(result, Some(("user".into(), "repo".into())));
    }

    #[test]
    fn test_parse_https_url() {
        let result = parse_owner_repo("https://github.com/user/repo.git");
        assert_eq!(result, Some(("user".into(), "repo".into())));
    }

    #[test]
    fn test_parse_https_url_no_git_suffix() {
        let result = parse_owner_repo("https://github.com/user/repo");
        assert_eq!(result, Some(("user".into(), "repo".into())));
    }

    #[test]
    fn test_parse_http_url() {
        let result = parse_owner_repo("http://gitlab.com/org/project.git");
        assert_eq!(result, Some(("org".into(), "project".into())));
    }

    #[test]
    fn test_parse_invalid_url_returns_none() {
        assert_eq!(parse_owner_repo("not-a-url"), None);
        assert_eq!(parse_owner_repo(""), None);
        assert_eq!(parse_owner_repo("https://github.com/"), None);
        assert_eq!(parse_owner_repo("git@github.com:"), None);
    }

    #[test]
    fn test_parse_trailing_slash() {
        let result = parse_owner_repo("https://github.com/user/repo/");
        assert_eq!(result, Some(("user".into(), "repo".into())));
    }

    // === GitHub URL generation tests ===

    #[test]
    fn test_github_repo_web_url_ssh() {
        let gh = GitHubProvider::new();
        assert_eq!(
            gh.repo_web_url("git@github.com:user/repo.git"),
            Some("https://github.com/user/repo".into())
        );
    }

    #[test]
    fn test_github_repo_web_url_https() {
        let gh = GitHubProvider::new();
        assert_eq!(
            gh.repo_web_url("https://github.com/user/repo.git"),
            Some("https://github.com/user/repo".into())
        );
    }

    #[test]
    fn test_github_commit_web_url() {
        let gh = GitHubProvider::new();
        assert_eq!(
            gh.commit_web_url("git@github.com:user/repo.git", "abc123"),
            Some("https://github.com/user/repo/commit/abc123".into())
        );
    }

    #[test]
    fn test_github_branch_web_url() {
        let gh = GitHubProvider::new();
        assert_eq!(
            gh.branch_web_url("git@github.com:user/repo.git", "main"),
            Some("https://github.com/user/repo/tree/main".into())
        );
    }

    #[test]
    fn test_github_url_invalid_remote() {
        let gh = GitHubProvider::new();
        assert_eq!(gh.repo_web_url("not-a-url"), None);
        assert_eq!(gh.commit_web_url("not-a-url", "abc"), None);
        assert_eq!(gh.branch_web_url("not-a-url", "main"), None);
    }

    // === GitLab URL generation tests ===

    #[test]
    fn test_gitlab_repo_web_url_ssh() {
        let gl = GitLabProvider::new();
        assert_eq!(
            gl.repo_web_url("git@gitlab.com:org/project.git"),
            Some("https://gitlab.com/org/project".into())
        );
    }

    #[test]
    fn test_gitlab_repo_web_url_https() {
        let gl = GitLabProvider::new();
        assert_eq!(
            gl.repo_web_url("https://gitlab.com/org/project.git"),
            Some("https://gitlab.com/org/project".into())
        );
    }

    #[test]
    fn test_gitlab_commit_web_url() {
        let gl = GitLabProvider::new();
        assert_eq!(
            gl.commit_web_url("git@gitlab.com:org/project.git", "def456"),
            Some("https://gitlab.com/org/project/-/commit/def456".into())
        );
    }

    #[test]
    fn test_gitlab_branch_web_url() {
        let gl = GitLabProvider::new();
        assert_eq!(
            gl.branch_web_url("git@gitlab.com:org/project.git", "develop"),
            Some("https://gitlab.com/org/project/-/tree/develop".into())
        );
    }

    #[test]
    fn test_gitlab_url_invalid_remote() {
        let gl = GitLabProvider::new();
        assert_eq!(gl.repo_web_url("not-a-url"), None);
        assert_eq!(gl.commit_web_url("not-a-url", "abc"), None);
        assert_eq!(gl.branch_web_url("not-a-url", "main"), None);
    }

    // === HostIntegration facade tests ===

    #[test]
    fn test_host_integration_delegates_url_generation() {
        let integration = HostIntegration::new(Box::new(GitHubProvider::new()));
        assert_eq!(
            integration.repo_web_url("git@github.com:user/repo.git"),
            Some("https://github.com/user/repo".into())
        );
        assert_eq!(
            integration.commit_web_url("git@github.com:user/repo.git", "sha1"),
            Some("https://github.com/user/repo/commit/sha1".into())
        );
        assert_eq!(
            integration.branch_web_url("git@github.com:user/repo.git", "feat"),
            Some("https://github.com/user/repo/tree/feat".into())
        );
    }

    #[test]
    fn test_host_integration_stub_methods_return_not_implemented() {
        let mut integration = HostIntegration::new(Box::new(GitHubProvider::new()));

        let auth_err = integration.authenticate("token123").unwrap_err();
        assert!(matches!(auth_err, HostError::NotImplemented { .. }));

        let list_err = integration
            .list_pull_requests("https://github.com/u/r")
            .unwrap_err();
        assert!(matches!(list_err, HostError::NotImplemented { .. }));

        let create_err = integration
            .create_pull_request(CreatePrParams {
                title: "t".into(),
                description: "d".into(),
                source_branch: "s".into(),
                target_branch: "m".into(),
            })
            .unwrap_err();
        assert!(matches!(create_err, HostError::NotImplemented { .. }));
    }

    // === HostError -> GitError conversion test ===

    #[test]
    fn test_host_error_converts_to_git_error() {
        let err: GitError = HostError::AuthFailed {
            reason: "bad token".into(),
        }
        .into();
        assert!(matches!(err, GitError::AuthenticationFailed));

        let err: GitError = HostError::NotImplemented {
            message: "stub".into(),
        }
        .into();
        assert!(matches!(err, GitError::InvalidArgument(_)));

        let err: GitError = HostError::InvalidUrl {
            url: "bad".into(),
        }
        .into();
        assert!(matches!(err, GitError::InvalidArgument(_)));
    }
}

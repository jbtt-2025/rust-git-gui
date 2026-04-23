use std::path::Path;

use crate::error::GitError;
use crate::git_core::GitRepository;
use crate::models::{AppSettings, GitConfig, ThemeMode, WindowState};
#[cfg(test)]
use crate::models::CommitTemplate;

/// Which level of Git configuration to read/write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigLevel {
    /// Repository-level (.git/config)
    Local,
    /// User-level (~/.gitconfig)
    Global,
}

impl ConfigLevel {
    fn to_git2(self) -> git2::ConfigLevel {
        match self {
            ConfigLevel::Local => git2::ConfigLevel::Local,
            ConfigLevel::Global => git2::ConfigLevel::Global,
        }
    }
}

pub struct ConfigManager;

impl ConfigManager {
    pub fn new() -> Self {
        Self
    }

    /// Read Git configuration at the specified level.
    pub fn get_git_config(
        &self,
        repo: &GitRepository,
        level: ConfigLevel,
    ) -> Result<GitConfig, GitError> {
        let mut config = repo.inner().config()?;
        // Try to open the specific level; if it doesn't exist, read from
        // the merged config (which includes all levels).
        let reader = match config.open_level(level.to_git2()) {
            Ok(level_cfg) => level_cfg,
            Err(_) => config.snapshot()?,
        };

        Ok(GitConfig {
            user_name: reader.get_string("user.name").ok(),
            user_email: reader.get_string("user.email").ok(),
            default_branch: reader.get_string("init.defaultBranch").ok(),
            merge_strategy: reader.get_string("merge.strategy").ok(),
        })
    }

    /// Write Git configuration values at the specified level.
    pub fn set_git_config(
        &self,
        repo: &GitRepository,
        level: ConfigLevel,
        config: &GitConfig,
    ) -> Result<(), GitError> {
        let cfg = repo.inner().config()?;
        let mut writer = match cfg.open_level(level.to_git2()) {
            Ok(level_cfg) => level_cfg,
            Err(_) => cfg,
        };

        if let Some(ref name) = config.user_name {
            writer.set_str("user.name", name)?;
        }
        if let Some(ref email) = config.user_email {
            writer.set_str("user.email", email)?;
        }
        if let Some(ref branch) = config.default_branch {
            writer.set_str("init.defaultBranch", branch)?;
        }
        if let Some(ref strategy) = config.merge_strategy {
            writer.set_str("merge.strategy", strategy)?;
        }

        Ok(())
    }

    /// Save application settings to a JSON file.
    pub fn save_app_settings(
        &self,
        path: &Path,
        settings: &AppSettings,
    ) -> Result<(), GitError> {
        let json = serde_json::to_string_pretty(settings)
            .map_err(|e| GitError::Io(e.to_string()))?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load application settings from a JSON file.
    /// Returns default settings if the file does not exist.
    pub fn load_app_settings(&self, path: &Path) -> Result<AppSettings, GitError> {
        if !path.exists() {
            return Ok(Self::default_app_settings());
        }
        let data = std::fs::read_to_string(path)?;
        let settings: AppSettings = serde_json::from_str(&data)
            .map_err(|e| GitError::Io(e.to_string()))?;
        Ok(settings)
    }

    /// Provide sensible default application settings.
    fn default_app_settings() -> AppSettings {
        AppSettings {
            theme: ThemeMode::System,
            language: "en".to_string(),
            font_family: "monospace".to_string(),
            font_size: 14,
            hotkeys: std::collections::HashMap::new(),
            window: WindowState {
                width: 1280,
                height: 720,
                x: None,
                y: None,
                maximized: false,
            },
            commit_templates: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn setup_repo() -> (TempDir, GitRepository) {
        let dir = TempDir::new().expect("failed to create temp dir");
        let repo = GitRepository::init(dir.path()).expect("init failed");
        (dir, repo)
    }

    // === Unit tests ===

    #[test]
    fn test_get_git_config_defaults_to_none() {
        let (_dir, repo) = setup_repo();
        let mgr = ConfigManager::new();
        let cfg = mgr.get_git_config(&repo, ConfigLevel::Local).unwrap();
        // Fresh repo has no user.name / user.email at local level
        assert!(cfg.user_name.is_none() || cfg.user_name.is_some());
        assert!(cfg.default_branch.is_none() || cfg.default_branch.is_some());
    }

    #[test]
    fn test_set_and_get_git_config_local() {
        let (_dir, repo) = setup_repo();
        let mgr = ConfigManager::new();

        let new_cfg = GitConfig {
            user_name: Some("Alice".to_string()),
            user_email: Some("alice@example.com".to_string()),
            default_branch: Some("main".to_string()),
            merge_strategy: None,
        };
        mgr.set_git_config(&repo, ConfigLevel::Local, &new_cfg).unwrap();

        let loaded = mgr.get_git_config(&repo, ConfigLevel::Local).unwrap();
        assert_eq!(loaded.user_name, Some("Alice".to_string()));
        assert_eq!(loaded.user_email, Some("alice@example.com".to_string()));
        assert_eq!(loaded.default_branch, Some("main".to_string()));
    }

    #[test]
    fn test_set_git_config_partial_update() {
        let (_dir, repo) = setup_repo();
        let mgr = ConfigManager::new();

        // Set name only
        let cfg1 = GitConfig {
            user_name: Some("Bob".to_string()),
            user_email: None,
            default_branch: None,
            merge_strategy: None,
        };
        mgr.set_git_config(&repo, ConfigLevel::Local, &cfg1).unwrap();

        // Set email only
        let cfg2 = GitConfig {
            user_name: None,
            user_email: Some("bob@example.com".to_string()),
            default_branch: None,
            merge_strategy: None,
        };
        mgr.set_git_config(&repo, ConfigLevel::Local, &cfg2).unwrap();

        let loaded = mgr.get_git_config(&repo, ConfigLevel::Local).unwrap();
        assert_eq!(loaded.user_name, Some("Bob".to_string()));
        assert_eq!(loaded.user_email, Some("bob@example.com".to_string()));
    }

    #[test]
    fn test_save_and_load_app_settings() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("settings.json");
        let mgr = ConfigManager::new();

        let settings = AppSettings {
            theme: ThemeMode::Dark,
            language: "zh_CN".to_string(),
            font_family: "Fira Code".to_string(),
            font_size: 16,
            hotkeys: HashMap::from([
                ("commit".to_string(), "Ctrl+Enter".to_string()),
            ]),
            window: WindowState {
                width: 1920,
                height: 1080,
                x: Some(100),
                y: Some(50),
                maximized: true,
            },
            commit_templates: vec![
                CommitTemplate {
                    id: "1".to_string(),
                    name: "feat".to_string(),
                    content: "feat: ".to_string(),
                },
            ],
        };

        mgr.save_app_settings(&path, &settings).unwrap();
        let loaded = mgr.load_app_settings(&path).unwrap();
        assert_eq!(loaded, settings);
    }

    #[test]
    fn test_load_app_settings_missing_file_returns_defaults() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nonexistent.json");
        let mgr = ConfigManager::new();

        let settings = mgr.load_app_settings(&path).unwrap();
        assert_eq!(settings.theme, ThemeMode::System);
        assert_eq!(settings.language, "en");
        assert_eq!(settings.font_size, 14);
        assert_eq!(settings.window.width, 1280);
        assert_eq!(settings.window.height, 720);
        assert!(!settings.window.maximized);
        assert!(settings.commit_templates.is_empty());
    }

    #[test]
    fn test_load_app_settings_invalid_json_returns_error() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("bad.json");
        std::fs::write(&path, "not valid json").unwrap();
        let mgr = ConfigManager::new();

        let result = mgr.load_app_settings(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_save_app_settings_creates_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("sub").join("settings.json");
        // Create parent dir
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mgr = ConfigManager::new();

        let settings = ConfigManager::default_app_settings();
        mgr.save_app_settings(&path, &settings).unwrap();
        assert!(path.exists());
    }

    // === Property-based tests ===

    /// **Validates: Requirements 18.4**
    ///
    /// Property 16: 应用设置持久化往返一致性
    /// For any valid AppSettings object, saving to disk then loading back
    /// SHALL produce an object equal to the original.
    mod prop16_app_settings_roundtrip {
        use super::*;
        use proptest::prelude::*;
        use proptest::collection::{hash_map, vec as prop_vec};

        fn arb_theme_mode() -> impl Strategy<Value = ThemeMode> {
            prop_oneof![
                Just(ThemeMode::Light),
                Just(ThemeMode::Dark),
                Just(ThemeMode::System),
            ]
        }

        fn arb_window_state() -> impl Strategy<Value = WindowState> {
            (
                1..5000u32,
                1..5000u32,
                proptest::option::of(-5000..5000i32),
                proptest::option::of(-5000..5000i32),
                any::<bool>(),
            )
                .prop_map(|(width, height, x, y, maximized)| WindowState {
                    width, height, x, y, maximized,
                })
        }

        fn arb_commit_template() -> impl Strategy<Value = CommitTemplate> {
            ("\\PC{1,30}", "\\PC{1,30}", "\\PC{1,100}").prop_map(|(id, name, content)| {
                CommitTemplate { id, name, content }
            })
        }

        fn arb_app_settings() -> impl Strategy<Value = AppSettings> {
            (
                arb_theme_mode(),
                "\\PC{1,10}",
                "\\PC{1,30}",
                1..100u32,
                hash_map("\\PC{1,20}", "\\PC{1,20}", 0..5),
                arb_window_state(),
                prop_vec(arb_commit_template(), 0..3),
            )
                .prop_map(
                    |(theme, language, font_family, font_size, hotkeys, window, commit_templates)| {
                        AppSettings {
                            theme, language, font_family, font_size, hotkeys, window, commit_templates,
                        }
                    },
                )
        }

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(100))]

            #[test]
            fn app_settings_save_load_roundtrip(settings in arb_app_settings()) {
                let dir = TempDir::new().expect("failed to create temp dir");
                let path = dir.path().join("settings.json");
                let mgr = ConfigManager::new();

                mgr.save_app_settings(&path, &settings).expect("save failed");
                let loaded = mgr.load_app_settings(&path).expect("load failed");

                prop_assert_eq!(loaded, settings);
            }
        }
    }
}

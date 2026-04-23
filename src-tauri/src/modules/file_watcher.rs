use std::collections::HashMap;
use std::path::Path;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};

use crate::error::GitError;
use crate::models::{FileChangeEvent, TabId};

/// Callback invoked when debounced file changes are detected.
pub type FileChangeCallback = Arc<dyn Fn(FileChangeEvent) + Send + Sync>;

/// Holds the watcher and a channel to signal the debounce thread to stop.
struct WatchEntry {
    _watcher: RecommendedWatcher,
    stop_tx: Sender<()>,
}

/// Watches working directories for file changes and emits debounced
/// `FileChangeEvent`s via a callback. Each tab can watch one directory.
pub struct FileWatcher {
    watchers: HashMap<TabId, WatchEntry>,
}

impl FileWatcher {
    pub fn new() -> Self {
        Self {
            watchers: HashMap::new(),
        }
    }

    /// Start watching `path` for file changes associated with `tab_id`.
    /// Changed paths are accumulated and flushed to `callback` every 2 seconds.
    pub fn watch(
        &mut self,
        tab_id: TabId,
        path: &Path,
        callback: FileChangeCallback,
    ) -> Result<(), GitError> {
        // If already watching this tab, stop the old watcher first.
        self.unwatch(&tab_id);

        let tab_id_clone = tab_id.clone();
        let pending: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let pending_for_watcher = pending.clone();
        let watched_root = path.to_path_buf();

        // Create the notify watcher with a closure that accumulates changed paths.
        let mut watcher = RecommendedWatcher::new(
            move |result: Result<Event, notify::Error>| {
                if let Ok(event) = result {
                    let mut paths = pending_for_watcher.lock().unwrap();
                    for p in &event.paths {
                        let display = p
                            .strip_prefix(&watched_root)
                            .unwrap_or(p)
                            .to_string_lossy()
                            .into_owned();
                        if !paths.contains(&display) {
                            paths.push(display);
                        }
                    }
                }
            },
            Config::default(),
        )
        .map_err(|e| GitError::Io(e.to_string()))?;

        watcher
            .watch(path, RecursiveMode::Recursive)
            .map_err(|e| GitError::Io(e.to_string()))?;

        // Channel to signal the debounce thread to stop.
        let (stop_tx, stop_rx) = mpsc::channel::<()>();

        // Spawn a debounce thread that flushes accumulated paths every 2 seconds.
        let pending_for_thread = pending.clone();
        std::thread::spawn(move || {
            loop {
                // Sleep for 2 seconds, or break if stop signal received.
                match stop_rx.recv_timeout(Duration::from_secs(2)) {
                    Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        // Flush accumulated paths.
                        let paths: Vec<String> = {
                            let mut guard = pending_for_thread.lock().unwrap();
                            if guard.is_empty() {
                                continue;
                            }
                            std::mem::take(&mut *guard)
                        };
                        callback(FileChangeEvent {
                            tab_id: tab_id_clone.clone(),
                            changed_paths: paths,
                        });
                    }
                }
            }
        });

        self.watchers.insert(
            tab_id,
            WatchEntry {
                _watcher: watcher,
                stop_tx,
            },
        );

        Ok(())
    }

    /// Stop watching the directory associated with `tab_id`.
    pub fn unwatch(&mut self, tab_id: &TabId) {
        if let Some(entry) = self.watchers.remove(tab_id) {
            // Signal the debounce thread to stop. Ignore send errors
            // (thread may have already exited).
            let _ = entry.stop_tx.send(());
        }
    }

    /// Returns true if the given tab is currently being watched.
    pub fn is_watching(&self, tab_id: &TabId) -> bool {
        self.watchers.contains_key(tab_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tempfile::TempDir;

    fn temp_dir() -> TempDir {
        TempDir::new().expect("failed to create temp dir")
    }

    #[test]
    fn test_watch_and_unwatch() {
        let dir = temp_dir();
        let mut fw = FileWatcher::new();
        let tab = TabId("tab-1".to_string());
        let cb: FileChangeCallback = Arc::new(|_| {});

        fw.watch(tab.clone(), dir.path(), cb).unwrap();
        assert!(fw.is_watching(&tab));

        fw.unwatch(&tab);
        assert!(!fw.is_watching(&tab));
    }

    #[test]
    fn test_unwatch_nonexistent_is_noop() {
        let mut fw = FileWatcher::new();
        let tab = TabId("no-such-tab".to_string());
        fw.unwatch(&tab); // should not panic
    }

    #[test]
    fn test_rewatch_replaces_old_watcher() {
        let dir = temp_dir();
        let mut fw = FileWatcher::new();
        let tab = TabId("tab-1".to_string());

        let count = Arc::new(AtomicUsize::new(0));
        let count_clone = count.clone();
        let cb1: FileChangeCallback = Arc::new(move |_| {
            count_clone.fetch_add(1, Ordering::Relaxed);
        });
        let cb2: FileChangeCallback = Arc::new(|_| {});

        fw.watch(tab.clone(), dir.path(), cb1).unwrap();
        // Re-watch with a different callback — old watcher should be dropped.
        fw.watch(tab.clone(), dir.path(), cb2).unwrap();
        assert!(fw.is_watching(&tab));
    }

    #[test]
    fn test_detects_file_changes() {
        let dir = temp_dir();
        let mut fw = FileWatcher::new();
        let tab = TabId("tab-detect".to_string());

        let events: Arc<Mutex<Vec<FileChangeEvent>>> = Arc::new(Mutex::new(Vec::new()));
        let events_clone = events.clone();
        let cb: FileChangeCallback = Arc::new(move |evt| {
            events_clone.lock().unwrap().push(evt);
        });

        fw.watch(tab.clone(), dir.path(), cb).unwrap();

        // Create a file to trigger a change event.
        fs::write(dir.path().join("hello.txt"), "world").unwrap();

        // Wait for the debounce interval (2s) plus a small buffer.
        std::thread::sleep(Duration::from_millis(3000));

        let collected = events.lock().unwrap();
        assert!(
            !collected.is_empty(),
            "Expected at least one FileChangeEvent after file creation"
        );
        assert_eq!(collected[0].tab_id, tab);
        assert!(
            collected[0]
                .changed_paths
                .iter()
                .any(|p| p.contains("hello.txt")),
            "Expected changed_paths to contain hello.txt, got: {:?}",
            collected[0].changed_paths
        );

        // Cleanup
        drop(collected);
        fw.unwatch(&tab);
    }
}

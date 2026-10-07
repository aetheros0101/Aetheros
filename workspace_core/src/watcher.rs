use crate::error::{Result, WorkspaceError};
use crate::models::{WatchEventKind, WorkspaceWatchEvent};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

pub struct WorkspaceWatcher {
    _watcher: RecommendedWatcher,
    receiver: Receiver<Result<WorkspaceWatchEvent>>,
}

impl WorkspaceWatcher {
    pub fn start(root: PathBuf) -> Result<Self> {
        let (tx, rx) = mpsc::channel();
        let callback_root = root.clone();
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            let mapped = res
                .map(|event| map_event(event, &callback_root))
                .map_err(|e| WorkspaceError::Watcher(e.to_string()));
            let _ = tx.send(mapped);
        })
        .map_err(|e| WorkspaceError::Watcher(e.to_string()))?;

        watcher
            .watch(&root, RecursiveMode::Recursive)
            .map_err(|e| WorkspaceError::Watcher(e.to_string()))?;

        Ok(Self {
            _watcher: watcher,
            receiver: rx,
        })
    }

    pub fn try_next(&self) -> Option<Result<WorkspaceWatchEvent>> {
        self.receiver.try_recv().ok()
    }

    /// Collect a short burst of filesystem events.
    pub fn drain(&self, wait: Duration, max_events: usize) -> Vec<Result<WorkspaceWatchEvent>> {
        if max_events == 0 {
            return Vec::new();
        }
        let mut events = Vec::with_capacity(max_events.min(32));
        if let Ok(first) = self.receiver.recv_timeout(wait) {
            events.push(first);
        } else {
            return events;
        }
        while events.len() < max_events {
            match self.receiver.try_recv() {
                Ok(event) => events.push(event),
                Err(_) => break,
            }
        }
        events
    }

    /// Drain + coalesce by path (last event wins per path). Rename keeps old_path.
    pub fn drain_coalesced(
        &self,
        wait: Duration,
        max_events: usize,
    ) -> Vec<WorkspaceWatchEvent> {
        let raw = self.drain(wait, max_events);
        coalesce_events(raw.into_iter().filter_map(|r| r.ok()).collect())
    }
}

/// Merge events so each path appears once (latest kind wins).
pub fn coalesce_events(events: Vec<WorkspaceWatchEvent>) -> Vec<WorkspaceWatchEvent> {
    let mut map: HashMap<String, WorkspaceWatchEvent> = HashMap::new();
    for ev in events {
        let key = if !ev.path.is_empty() {
            ev.path.clone()
        } else {
            ev.old_path.clone().unwrap_or_default()
        };
        if key.is_empty() {
            continue;
        }
        if let Some(existing) = map.get_mut(&key) {
            // Removed beats everything; Created after Removed → Modified-ish → Created
            match (&existing.kind, &ev.kind) {
                (_, WatchEventKind::Removed) => *existing = ev,
                (WatchEventKind::Removed, WatchEventKind::Created) => {
                    existing.kind = WatchEventKind::Modified;
                    existing.path = ev.path;
                }
                (WatchEventKind::Created, WatchEventKind::Modified) => {
                    // stay Created
                }
                _ => *existing = ev,
            }
        } else {
            map.insert(key, ev);
        }
    }
    map.into_values().collect()
}

fn map_event(event: Event, root: &Path) -> WorkspaceWatchEvent {
    let kind = match event.kind {
        EventKind::Create(_) => WatchEventKind::Created,
        EventKind::Modify(notify::event::ModifyKind::Name(_)) => WatchEventKind::Renamed,
        EventKind::Modify(_) => WatchEventKind::Modified,
        EventKind::Remove(_) => WatchEventKind::Removed,
        _ => WatchEventKind::Other,
    };
    let path = event
        .paths
        .first()
        .and_then(|p| p.strip_prefix(root).ok())
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default();
    let old_path = event
        .paths
        .get(1)
        .and_then(|p| p.strip_prefix(root).ok())
        .map(|p| p.to_string_lossy().replace('\\', "/"));
    WorkspaceWatchEvent {
        kind,
        path,
        old_path,
    }
}

/// Simple debounce helper for callers that poll the watcher.
pub struct Debouncer {
    last: HashMap<String, Instant>,
    window: Duration,
}

impl Debouncer {
    pub fn new(window: Duration) -> Self {
        Self {
            last: HashMap::new(),
            window,
        }
    }

    /// Returns true if this path should be processed now.
    pub fn should_process(&mut self, path: &str) -> bool {
        let now = Instant::now();
        if let Some(prev) = self.last.get(path) {
            if now.duration_since(*prev) < self.window {
                return false;
            }
        }
        self.last.insert(path.to_string(), now);
        true
    }

    pub fn clear(&mut self) {
        self.last.clear();
    }
}

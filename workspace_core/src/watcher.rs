use crate::{error::{Result, WorkspaceError}, models::{WatchEventKind, WorkspaceWatchEvent}};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::{path::{Path, PathBuf}, sync::mpsc::{self, Receiver}, time::Duration};

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
        }).map_err(|e| WorkspaceError::Watcher(e.to_string()))?;

        watcher.watch(&root, RecursiveMode::Recursive)
            .map_err(|e| WorkspaceError::Watcher(e.to_string()))?;

        Ok(Self { _watcher: watcher, receiver: rx })
    }

    pub fn try_next(&self) -> Option<Result<WorkspaceWatchEvent>> { self.receiver.try_recv().ok() }

    /// Collects a short burst of filesystem events. The caller can coalesce
    /// events by path without losing events belonging to other files.
    pub fn drain(&self, wait: Duration, max_events: usize) -> Vec<Result<WorkspaceWatchEvent>> {
        if max_events == 0 { return Vec::new(); }
        let mut events = Vec::with_capacity(max_events.min(32));
        if let Ok(first) = self.receiver.recv_timeout(wait) { events.push(first); } else { return events; }
        while events.len() < max_events {
            match self.receiver.try_recv() {
                Ok(event) => events.push(event),
                Err(_) => break,
            }
        }
        events
    }
}

fn map_event(event: Event, root: &Path) -> WorkspaceWatchEvent {
    let kind = match event.kind {
        EventKind::Create(_) => WatchEventKind::Created,
        EventKind::Modify(notify::event::ModifyKind::Name(_)) => WatchEventKind::Renamed,
        EventKind::Modify(_) => WatchEventKind::Modified,
        EventKind::Remove(_) => WatchEventKind::Removed,
        _ => WatchEventKind::Other,
    };
    let path = event.paths.first().and_then(|p| p.strip_prefix(root).ok())
        .map(|p| p.to_string_lossy().replace('\\', "/")).unwrap_or_default();
    let old_path = event.paths.get(1).and_then(|p| p.strip_prefix(root).ok())
        .map(|p| p.to_string_lossy().replace('\\', "/"));
    WorkspaceWatchEvent { kind, path, old_path }
}

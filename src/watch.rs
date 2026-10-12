use crate::{Entry, Event};
use notify::{RecursiveMode, Watcher};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    time::{Duration, Instant},
};

pub struct Watch(Arc<AtomicBool>);
impl Drop for Watch {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Relaxed);
    }
}
fn scan(root: &std::path::Path) -> Vec<Entry> {
    walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            !matches!(
                e.file_name().to_str(),
                Some(".git" | "node_modules" | "target" | ".ssh")
            )
        })
        .filter_map(Result::ok)
        .filter_map(|e| {
            let m = e.metadata().ok()?;
            Some(Entry {
                path: e.into_path(),
                directory: m.is_dir(),
                size: m.len(),
            })
        })
        .collect()
}
pub fn start(root: PathBuf, generation: u64, tx: mpsc::Sender<Event>) -> Watch {
    let stop = Arc::new(AtomicBool::new(false));
    let stopping = stop.clone();
    std::thread::spawn(move || {
        let (changes, receiver) = mpsc::sync_channel(64);
        let watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
            let _ = changes.try_send(event);
        });
        let mut watcher = match watcher {
            Ok(w) => Some(w),
            Err(e) => {
                let _ = tx.send(Event::WatchError(generation, e.to_string()));
                None
            }
        };
        if let Some(w) = &mut watcher {
            if let Err(e) = w.watch(&root, RecursiveMode::Recursive) {
                let _ = tx.send(Event::WatchError(generation, e.to_string()));
                watcher = None;
            }
        }
        if tx
            .send(Event::Indexed(generation, scan(&root), true))
            .is_err()
        {
            return;
        }
        if watcher.is_none() {
            return;
        }
        let mut pending = None;
        while !stopping.load(Ordering::Relaxed) {
            match receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(Ok(event)) if !event.kind.is_access() => {
                    pending = Some(pending.unwrap_or_else(Instant::now));
                }
                Ok(Err(e)) => {
                    let _ = tx.send(Event::WatchError(generation, e.to_string()));
                    pending = Some(Instant::now());
                }
                _ => (),
            }
            // Bound latency during continuous writes rather than waiting indefinitely for quiet.
            if pending.is_some_and(|since| since.elapsed() >= Duration::from_millis(500)) {
                pending = None;
                if tx
                    .send(Event::Indexed(generation, scan(&root), false))
                    .is_err()
                {
                    break;
                }
            }
        }
    });
    Watch(stop)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn wait_for(rx: &mpsc::Receiver<Event>, filename: &str, present: bool) {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            assert!(!remaining.is_zero(), "watcher failed to reflect {filename}");
            match rx.recv_timeout(remaining).expect("watcher update") {
                Event::Indexed(_, entries, _)
                    if entries
                        .iter()
                        .any(|e| e.path.file_name().is_some_and(|n| n == filename))
                        == present =>
                {
                    return
                }
                Event::WatchError(_, e) => panic!("watcher error: {e}"),
                _ => (),
            }
        }
    }
    #[test]
    fn native_watcher_tracks_create_rename_and_delete() {
        let root = tempfile::tempdir().unwrap();
        let (tx, rx) = mpsc::channel();
        let _watcher = start(root.path().to_owned(), 1, tx);
        wait_for(&rx, "new.txt", false);
        std::fs::write(root.path().join("new.txt"), "content").unwrap();
        wait_for(&rx, "new.txt", true);
        std::fs::rename(root.path().join("new.txt"), root.path().join("renamed.txt")).unwrap();
        wait_for(&rx, "renamed.txt", true);
        std::fs::remove_file(root.path().join("renamed.txt")).unwrap();
        wait_for(&rx, "renamed.txt", false);
    }
}

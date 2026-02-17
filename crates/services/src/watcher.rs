//! File watcher service using inotify.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use inotify::{EventMask, Inotify, WatchMask};
use tokio::sync::mpsc;

const DEBOUNCE_MS: u64 = 200;

pub struct FileWatcher;

impl FileWatcher {
    pub fn watch(path: PathBuf) -> mpsc::UnboundedReceiver<()> {
        let (tx, rx) = mpsc::unbounded_channel();
        thread::spawn(move || {
            if let Err(err) = watch_file(path, tx) {
                tracing::warn!("File watcher stopped: {}", err);
            }
        });
        rx
    }
}

fn watch_file(path: PathBuf, tx: mpsc::UnboundedSender<()>) -> anyhow::Result<()> {
    let mut inotify = Inotify::init()?;
    let (watch_dir, watch_name) = watch_target(&path)?;

    inotify.watches().add(
        watch_dir,
        WatchMask::MODIFY
            | WatchMask::CLOSE_WRITE
            | WatchMask::CREATE
            | WatchMask::DELETE
            | WatchMask::MOVED_TO
            | WatchMask::MOVE_SELF
            | WatchMask::DELETE_SELF,
    )?;

    let mut buffer = [0u8; 4096];
    let mut last_sent: Option<Instant> = None;

    loop {
        let events = inotify.read_events_blocking(&mut buffer)?;
        let mut should_reload = false;

        for event in events {
            let renamed_or_deleted = event.mask.contains(EventMask::MOVE_SELF)
                || event.mask.contains(EventMask::DELETE_SELF);
            let same_file = event
                .name
                .map(|name| name == watch_name.as_os_str())
                .unwrap_or(false);
            if renamed_or_deleted || same_file {
                should_reload = true;
                break;
            }
        }

        if should_reload {
            let now = Instant::now();
            let debounce_elapsed = last_sent
                .map(|last| now.duration_since(last) >= Duration::from_millis(DEBOUNCE_MS))
                .unwrap_or(true);
            if debounce_elapsed {
                if tx.send(()).is_err() {
                    break;
                }
                last_sent = Some(now);
            }
        }
    }

    Ok(())
}

fn watch_target(path: &Path) -> anyhow::Result<(&Path, OsString)> {
    let parent = path.parent().ok_or_else(|| {
        anyhow::anyhow!(
            "Invalid config path has no parent directory: {}",
            path.display()
        )
    })?;
    let name = path.file_name().ok_or_else(|| {
        anyhow::anyhow!("Invalid config path has no filename: {}", path.display())
    })?;
    Ok((parent, name.to_os_string()))
}

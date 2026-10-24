//! File watcher service using inotify.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

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

    loop {
        let events = inotify.read_events_blocking(&mut buffer)?;
        if !events
            .into_iter()
            .any(|event| concerns(&event, &watch_name))
        {
            continue;
        }

        // Editors save by truncating or by delete-and-rename, so the first
        // event often shows an empty or missing file. Wait for the burst to
        // end and drop what piled up, so the reload reads the final write.
        thread::sleep(Duration::from_millis(DEBOUNCE_MS));
        loop {
            match inotify.read_events(&mut buffer) {
                Ok(events) => {
                    if events.count() == 0 {
                        break;
                    }
                }
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(err) => return Err(err.into()),
            }
        }

        if tx.send(()).is_err() {
            break;
        }
    }

    Ok(())
}

fn concerns(event: &inotify::Event<&OsStr>, watch_name: &OsStr) -> bool {
    event.mask.contains(EventMask::MOVE_SELF)
        || event.mask.contains(EventMask::DELETE_SELF)
        || event.name.is_some_and(|name| name == watch_name)
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

//! Downscaled image previews for list rows, decoded off the UI thread.
//!
//! Handing a path to `img()` makes gpui decode the file at full size into an
//! asset cache that is never evicted. [`ThumbnailCache`] decodes once on the
//! background executor, shrinks to the slot's device-pixel size, keeps a PNG
//! copy on disk, and holds the result in a byte-bounded LRU whose evictions go
//! through `App::drop_image` so their sprite-atlas tiles are freed too.

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use gpui::{App, EntityId, Global, Pixels, RenderImage, Size};
use image::{DynamicImage, Frame, ImageFormat, ImageReader, RgbaImage};

/// Default memory budget: about 300 wallpaper previews at 2x.
pub const DEFAULT_THUMBNAIL_BUDGET: usize = 32 * 1024 * 1024;

/// Raster formats the cache decodes; anything else is left to the caller.
const SUPPORTED_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "bmp", "webp"];

/// Bumped when the on-disk encoding changes, so old files simply miss.
const DISK_FORMAT_VERSION: u8 = 1;

/// Whether `path` names a raster format the cache can thumbnail.
pub fn is_thumbnailable(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            SUPPORTED_EXTENSIONS
                .iter()
                .any(|supported| ext.eq_ignore_ascii_case(supported))
        })
}

/// Device-pixel box for a slot of `logical` size at `scale`.
pub fn target_size(logical: Size<Pixels>, scale: f32) -> (u32, u32) {
    let scale = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };
    let px = |v: Pixels| ((f32::from(v) * scale).ceil() as u32).max(1);
    (px(logical.width), px(logical.height))
}

/// Largest size within `max` that keeps the source aspect ratio, never
/// upscaling.
pub fn fit_within(src: (u32, u32), max: (u32, u32)) -> (u32, u32) {
    let (sw, sh) = src;
    let (mw, mh) = (max.0.max(1), max.1.max(1));
    if sw == 0 || sh == 0 {
        return (1, 1);
    }
    if sw <= mw && sh <= mh {
        return (sw, sh);
    }
    let ratio = (mw as f64 / sw as f64).min(mh as f64 / sh as f64);
    let w = ((sw as f64 * ratio).round() as u32).clamp(1, mw);
    let h = ((sh as f64 * ratio).round() as u32).clamp(1, mh);
    (w, h)
}

/// Identity of a thumbnail in memory and on disk: a changed mtime or size is
/// a different thumbnail.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ThumbnailKey {
    pub path: PathBuf,
    /// The source's mtime as the caller last saw it. `None` is resolved on
    /// the background thread for the disk name, but the memory entry then
    /// never notices the file changing.
    pub mtime: Option<SystemTime>,
    /// Device-pixel box to fit into; see [`target_size`].
    pub size: (u32, u32),
}

impl ThumbnailKey {
    pub fn new(path: impl Into<PathBuf>, mtime: Option<SystemTime>, size: (u32, u32)) -> Self {
        Self {
            path: path.into(),
            mtime,
            size,
        }
    }

    /// File name under the disk cache directory.
    pub fn file_name(&self) -> String {
        format!("{:016x}.png", self.hash())
    }

    // FNV-1a: stable across builds, unlike `DefaultHasher`.
    fn hash(&self) -> u64 {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        let mut feed = |bytes: &[u8]| {
            for byte in bytes {
                hash ^= u64::from(*byte);
                hash = hash.wrapping_mul(0x0100_0000_01b3);
            }
        };
        feed(&[DISK_FORMAT_VERSION]);
        feed(self.path.as_os_str().as_encoded_bytes());
        // Separator so a path suffix cannot collide with the numbers below.
        feed(&[0]);
        let nanos = self
            .mtime
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_nanos());
        feed(&nanos.to_le_bytes());
        feed(&self.size.0.to_le_bytes());
        feed(&self.size.1.to_le_bytes());
        hash
    }
}

/// `$XDG_CACHE_HOME/gpuishell/thumbs`, else `~/.cache/gpuishell/thumbs`.
pub fn default_disk_dir() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))?;
    Some(base.join("gpuishell").join("thumbs"))
}

/// What a row should draw this frame.
#[derive(Clone)]
pub enum Thumbnail {
    /// Not ready yet; draw a placeholder. The owner is notified when it lands.
    Loading,
    Ready(Arc<RenderImage>),
    /// Could not be decoded; draw the caller's fallback.
    Failed,
}

enum Slot {
    Loading(Vec<EntityId>),
    Ready {
        image: Arc<RenderImage>,
        bytes: usize,
        last_used: u64,
    },
    Failed,
}

struct State {
    entries: HashMap<ThumbnailKey, Slot>,
    budget: usize,
    bytes: usize,
    tick: u64,
    disk_dir: Option<PathBuf>,
}

impl State {
    /// Least-recently-used ready entries to drop so `bytes` fits the budget.
    fn evict(&mut self, keep: &ThumbnailKey) -> Vec<Arc<RenderImage>> {
        let mut evicted = Vec::new();
        while self.bytes > self.budget {
            let oldest = self
                .entries
                .iter()
                .filter(|(key, _)| *key != keep)
                .filter_map(|(key, slot)| match slot {
                    Slot::Ready { last_used, .. } => Some((*last_used, key.clone())),
                    _ => None,
                })
                .min_by_key(|(last_used, _)| *last_used);
            let Some((_, key)) = oldest else { break };
            if let Some(Slot::Ready { image, bytes, .. }) = self.entries.remove(&key) {
                self.bytes -= bytes;
                evicted.push(image);
            }
        }
        evicted
    }
}

/// Process-wide, byte-bounded LRU of downscaled images. Clones share state.
#[derive(Clone)]
pub struct ThumbnailCache(Rc<RefCell<State>>);

impl Global for ThumbnailCache {}

impl ThumbnailCache {
    pub fn new(budget_bytes: usize, disk_dir: Option<PathBuf>) -> Self {
        Self(Rc::new(RefCell::new(State {
            entries: HashMap::new(),
            budget: budget_bytes,
            bytes: 0,
            tick: 0,
            disk_dir,
        })))
    }

    /// Install the shared cache with the default budget and disk location.
    pub fn init(cx: &mut App) {
        if !cx.has_global::<Self>() {
            cx.set_global(Self::new(DEFAULT_THUMBNAIL_BUDGET, default_disk_dir()));
        }
    }

    pub fn global(cx: &App) -> Option<Self> {
        cx.try_global::<Self>().cloned()
    }

    /// The thumbnail for `key`, starting a background load on a miss.
    /// `owner` is notified when the load finishes.
    pub fn get(&self, key: &ThumbnailKey, owner: EntityId, cx: &App) -> Thumbnail {
        let disk_dir = {
            let mut state = self.0.borrow_mut();
            state.tick += 1;
            let tick = state.tick;
            match state.entries.get_mut(key) {
                Some(Slot::Ready {
                    image, last_used, ..
                }) => {
                    *last_used = tick;
                    return Thumbnail::Ready(image.clone());
                }
                Some(Slot::Failed) => return Thumbnail::Failed,
                Some(Slot::Loading(owners)) => {
                    if !owners.contains(&owner) {
                        owners.push(owner);
                    }
                    return Thumbnail::Loading;
                }
                None => {}
            }
            state
                .entries
                .insert(key.clone(), Slot::Loading(vec![owner]));
            state.disk_dir.clone()
        };

        let key = key.clone();
        let load_key = key.clone();
        let job = cx
            .background_executor()
            .spawn(async move { load_thumbnail(load_key, disk_dir.as_deref()) });
        let this = self.clone();
        cx.spawn(async move |cx| {
            let image = job
                .await
                .map(|buffer| Arc::new(RenderImage::new(vec![Frame::new(buffer)])));
            cx.update(|cx| this.finish(key, image, cx));
        })
        .detach();

        Thumbnail::Loading
    }

    fn finish(&self, key: ThumbnailKey, image: Option<Arc<RenderImage>>, cx: &mut App) {
        let (owners, evicted) = {
            let mut state = self.0.borrow_mut();
            let slot = match image {
                Some(image) => {
                    let dims = image.size(0);
                    let bytes = dims.width.0.max(0) as usize * dims.height.0.max(0) as usize * 4;
                    state.bytes += bytes;
                    state.tick += 1;
                    Slot::Ready {
                        image,
                        bytes,
                        last_used: state.tick,
                    }
                }
                None => Slot::Failed,
            };
            let owners = match state.entries.insert(key.clone(), slot) {
                Some(Slot::Loading(owners)) => owners,
                _ => Vec::new(),
            };
            (owners, state.evict(&key))
        };

        // Dropping the Arc alone leaves the atlas tile behind.
        for image in evicted {
            cx.drop_image(image, None);
        }
        for owner in owners {
            cx.notify(owner);
        }
    }
}

/// Disk hit or fresh decode, as BGRA ready for `RenderImage`.
fn load_thumbnail(mut key: ThumbnailKey, disk_dir: Option<&Path>) -> Option<RgbaImage> {
    if key.mtime.is_none() {
        key.mtime = std::fs::metadata(&key.path).and_then(|m| m.modified()).ok();
    }
    let (path, max) = (key.path.as_path(), key.size);
    let cached = disk_dir.map(|dir| dir.join(key.file_name()));

    let from_disk = cached.as_deref().and_then(|file| {
        let image = ImageReader::open(file).ok()?.decode();
        match image {
            Ok(image) => Some(image.into_rgba8()),
            Err(err) => {
                tracing::debug!("thumbnail cache entry {} unreadable: {err}", file.display());
                None
            }
        }
    });

    let mut rgba = match from_disk {
        Some(rgba) => rgba,
        None => {
            let rgba = decode_and_shrink(path, max)?;
            if let Some(file) = cached.as_deref() {
                write_disk_entry(file, &rgba);
            }
            rgba
        }
    };

    for pixel in rgba.as_chunks_mut::<4>().0 {
        pixel.swap(0, 2);
    }
    Some(rgba)
}

fn decode_and_shrink(path: &Path, max: (u32, u32)) -> Option<RgbaImage> {
    let decoded = ImageReader::open(path)
        .and_then(|reader| reader.with_guessed_format())
        .map_err(image::ImageError::IoError)
        .and_then(|reader| reader.decode());
    let image: DynamicImage = match decoded {
        Ok(image) => image,
        Err(err) => {
            tracing::debug!("thumbnail: cannot decode {}: {err}", path.display());
            return None;
        }
    };
    let (w, h) = fit_within((image.width(), image.height()), max);
    let image = if (w, h) == (image.width(), image.height()) {
        image
    } else {
        image.thumbnail_exact(w, h)
    };
    Some(image.into_rgba8())
}

fn write_disk_entry(file: &Path, rgba: &RgbaImage) {
    let Some(dir) = file.parent() else { return };
    let mut png = Vec::new();
    let result = std::fs::create_dir_all(dir)
        .map_err(image::ImageError::IoError)
        .and_then(|()| rgba.write_to(&mut Cursor::new(&mut png), ImageFormat::Png))
        .and_then(|()| {
            // Rename so a reader never sees a half-written file.
            let tmp = file.with_extension("png.tmp");
            std::fs::write(&tmp, &png)
                .and_then(|()| std::fs::rename(&tmp, file))
                .map_err(image::ImageError::IoError)
        });
    if let Err(err) = result {
        tracing::warn!("thumbnail: cannot write {}: {err}", file.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn key(mtime_secs: u64, size: (u32, u32)) -> ThumbnailKey {
        ThumbnailKey {
            path: PathBuf::from("/walls/forest.jpg"),
            mtime: Some(UNIX_EPOCH + Duration::from_secs(mtime_secs)),
            size,
        }
    }

    #[test]
    fn file_name_is_stable_and_png() {
        let name = key(100, (80, 56)).file_name();
        assert_eq!(name, key(100, (80, 56)).file_name());
        assert!(name.ends_with(".png"));
        assert_eq!(name.len(), 16 + ".png".len());
    }

    #[test]
    fn changed_mtime_misses() {
        assert_ne!(
            key(100, (80, 56)).file_name(),
            key(101, (80, 56)).file_name()
        );
    }

    #[test]
    fn changed_size_or_path_misses() {
        assert_ne!(
            key(100, (80, 56)).file_name(),
            key(100, (40, 28)).file_name()
        );
        let mut other = key(100, (80, 56));
        other.path = PathBuf::from("/walls/forest.png");
        assert_ne!(other.file_name(), key(100, (80, 56)).file_name());
    }

    #[test]
    fn fit_keeps_aspect_ratio() {
        // 16:9 into an 80x56 slot is width-bound.
        assert_eq!(fit_within((3888, 2187), (80, 56)), (80, 45));
        // Portrait is height-bound.
        assert_eq!(fit_within((1000, 2000), (80, 56)), (28, 56));
        assert_eq!(fit_within((512, 512), (56, 56)), (56, 56));
    }

    #[test]
    fn fit_never_upscales() {
        assert_eq!(fit_within((32, 32), (56, 56)), (32, 32));
        assert_eq!(fit_within((0, 10), (56, 56)), (1, 1));
    }

    #[test]
    fn target_size_applies_scale() {
        let slot = gpui::size(gpui::px(40.), gpui::px(28.));
        assert_eq!(target_size(slot, 2.0), (80, 56));
        assert_eq!(target_size(slot, 1.5), (60, 42));
        assert_eq!(target_size(slot, 0.0), (40, 28));
    }

    #[test]
    fn evicts_least_recently_used_over_budget() {
        let image = || Arc::new(RenderImage::new(vec![Frame::new(RgbaImage::new(4, 4))]));
        let cache = ThumbnailCache::new(2 * 64, None);
        let mut state = cache.0.borrow_mut();
        for (name, last_used) in [("a", 3), ("b", 1), ("c", 2)] {
            state.entries.insert(
                ThumbnailKey::new(name, None, (4, 4)),
                Slot::Ready {
                    image: image(),
                    bytes: 64,
                    last_used,
                },
            );
            state.bytes += 64;
        }
        let keep = ThumbnailKey::new("c", None, (4, 4));
        assert_eq!(state.evict(&keep).len(), 1);
        assert_eq!(state.bytes, 128);
        assert!(
            !state
                .entries
                .contains_key(&ThumbnailKey::new("b", None, (4, 4)))
        );
        assert!(state.entries.contains_key(&keep));
    }

    #[test]
    fn extensions_are_case_insensitive() {
        assert!(is_thumbnailable(Path::new("/a/b.JPG")));
        assert!(is_thumbnailable(Path::new("/a/b.webp")));
        assert!(!is_thumbnailable(Path::new("/a/b.svg")));
        assert!(!is_thumbnailable(Path::new("/a/b")));
    }
}

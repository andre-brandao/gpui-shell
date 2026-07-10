//! Registry for exclusive popup windows (panels, launcher).
//!
//! The shell only ever shows one interactive popup at a time: opening any
//! panel or the launcher closes whatever exclusive window is currently open.
//! Passive overlays (OSD, notification popups) intentionally do not
//! participate — a volume indicator must not close the control center.

use gpui::{AnyWindowHandle, App, AppContext as _, Global, Render, WindowHandle, WindowOptions};

/// Tracks the currently open exclusive window.
#[derive(Default)]
pub struct WindowRegistry {
    exclusive: Option<ExclusiveWindow>,
}

struct ExclusiveWindow {
    id: String,
    handle: AnyWindowHandle,
}

impl Global for WindowRegistry {}

impl WindowRegistry {
    /// Initialize the global registry. Call once at startup.
    pub fn init(cx: &mut App) {
        cx.set_global(WindowRegistry::default());
    }

    /// Toggle the exclusive window with the given ID.
    ///
    /// Closes whatever exclusive window is open. If that window had the same
    /// ID, this is a toggle-off and returns `false`; otherwise the new window
    /// is opened and this returns `true`.
    ///
    /// A tracked handle whose window is already gone (closed by the
    /// compositor or from inside the view) is treated as "was closed", so the
    /// new window still opens.
    pub fn toggle<V: Render + 'static>(
        id: &str,
        options: WindowOptions,
        cx: &mut App,
        build: impl FnOnce(&mut gpui::Context<V>) -> V + 'static,
    ) -> bool {
        let previous = cx.global_mut::<WindowRegistry>().exclusive.take();

        if let Some(previous) = previous {
            let was_alive = cx
                .update_window(previous.handle, |_, window, _| {
                    window.remove_window();
                })
                .is_ok();

            // Same window and it was actually open: plain toggle-off.
            if was_alive && previous.id == id {
                return false;
            }
        }

        if let Ok(handle) = cx.open_window(options, move |_, cx| cx.new(build)) {
            cx.global_mut::<WindowRegistry>().exclusive = Some(ExclusiveWindow {
                id: id.to_string(),
                handle: handle.into(),
            });
            true
        } else {
            false
        }
    }

    /// Get the typed handle of the exclusive window if `id` is currently open.
    pub fn active_handle<V: 'static>(id: &str, cx: &App) -> Option<WindowHandle<V>> {
        let registry = cx.try_global::<WindowRegistry>()?;
        let exclusive = registry.exclusive.as_ref()?;
        if exclusive.id == id {
            exclusive.handle.downcast::<V>()
        } else {
            None
        }
    }

    /// Drop tracking for `id` without touching the window.
    ///
    /// Call before a view closes its own window (e.g. launcher on Escape) so
    /// the registry doesn't keep a stale handle.
    pub fn window_closed(id: &str, cx: &mut App) {
        let registry = cx.global_mut::<WindowRegistry>();
        if registry
            .exclusive
            .as_ref()
            .is_some_and(|exclusive| exclusive.id == id)
        {
            registry.exclusive = None;
        }
    }
}

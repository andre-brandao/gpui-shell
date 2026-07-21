//! Dock UI: pinned + running app icons, shown as a standalone layer-shell
//! window per configured monitor.

pub mod config;
mod item;

pub use config::{DockConfig, DockHoverEffect, DockMonitors, DockVisibility};

use gpui::{
    AnyElement, App, Bounds, Context, DisplayId, Render, Size, Window, WindowBackgroundAppearance,
    WindowBounds, WindowKind, WindowOptions, div, img, layer_shell::*, point, prelude::*, px,
};
use ui::{ActiveTheme, radius, spacing};

use crate::config::{ActiveConfig, Config};
use crate::state::{AppState, display_id_for_window, record_window_display, watch};
use item::{DockItem, build_dock_items};

/// Retain only the windows on a resolved compositor monitor. When the monitor
/// cannot be resolved, retain every window so the dock remains useful.
fn windows_for_monitor(
    windows: &[services::Window],
    monitor_name: Option<&str>,
) -> Vec<services::Window> {
    match monitor_name {
        Some(monitor_name) => windows
            .iter()
            .filter(|window| window.monitor == monitor_name)
            .cloned()
            .collect(),
        None => windows.to_vec(),
    }
}

/// The dock's own view, rendered in a standalone layer-shell window.
struct Dock {
    _compositor: services::CompositorSubscriber,
    state: services::CompositorState,
}

impl Dock {
    fn new(cx: &mut Context<Self>) -> Self {
        let compositor = AppState::compositor(cx).clone();
        let state = compositor.get();

        watch(cx, compositor.subscribe(), |this, new_state, cx| {
            this.state = new_state;
            cx.notify();
        });

        Self {
            _compositor: compositor,
            state,
        }
    }

    fn current_monitor_name(&self, window: &Window, cx: &App) -> Option<String> {
        let display_id = display_id_for_window(window)?;
        let display_uuid = cx.find_display(display_id)?.uuid().ok()?;

        self.state
            .monitors
            .iter()
            .find(|monitor| {
                uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_DNS, monitor.name.as_bytes())
                    == display_uuid
            })
            .map(|monitor| monitor.name.clone())
    }

    fn items(&self, window: &Window, cx: &App) -> Vec<DockItem> {
        let apps = AppState::applications(cx).all();
        let scoped_windows = windows_for_monitor(
            &self.state.windows,
            self.current_monitor_name(window, cx).as_deref(),
        );
        build_dock_items(&scoped_windows, apps, &cx.config().dock.pinned)
    }

    fn render_item(&self, item: &DockItem, cx: &Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let icon_size = cx.config().dock.icon_size;

        div()
            .id(item.key.clone())
            .size(px(icon_size))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(radius::MD))
            .when_some(item.icon_path.clone(), |element, path| {
                element.child(img(path).size(px(icon_size * 0.75)))
            })
            .when(item.icon_path.is_none(), |element| {
                element.child(
                    div()
                        .text_size(theme.font_sizes.md)
                        .text_color(theme.text.primary)
                        .child(item.name.chars().take(1).collect::<String>()),
                )
            })
            .into_any_element()
    }
}

impl Render for Dock {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let is_vertical = cx.config().dock.position.is_vertical();
        let items = self.items(window, cx);
        let elements: Vec<AnyElement> = items
            .iter()
            .map(|item| self.render_item(item, cx))
            .collect();

        div()
            .id("dock")
            .flex()
            .when(is_vertical, |element| element.flex_col())
            .items_center()
            .justify_center()
            .gap(px(spacing::SM))
            .px(px(spacing::SM))
            .py(px(spacing::SM))
            .rounded(px(radius::LG))
            .bg(theme.bg.primary)
            .border_1()
            .border_color(theme.border.subtle)
            .children(elements)
    }
}

/// Window options for a dock window, sized to fit its content rather than
/// spanning the monitor like the bar does.
fn window_options(display_id: Option<DisplayId>, cx: &App) -> WindowOptions {
    let config = &cx.config().dock;
    let is_vertical = config.position.is_vertical();
    let item_count = AppState::compositor(cx)
        .get()
        .windows
        .len()
        .max(config.pinned.len())
        .max(1) as f32;
    let content_extent = item_count * (config.icon_size + spacing::SM) + spacing::SM;

    let (window_size, anchor) = if is_vertical {
        (
            Size::new(px(config.icon_size + spacing::SM * 2.0), px(content_extent)),
            match config.position {
                crate::bar::config::BarPosition::Left => Anchor::LEFT,
                _ => Anchor::RIGHT,
            },
        )
    } else {
        (
            Size::new(px(content_extent), px(config.icon_size + spacing::SM * 2.0)),
            match config.position {
                crate::bar::config::BarPosition::Top => Anchor::TOP,
                _ => Anchor::BOTTOM,
            },
        )
    };

    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: window_size,
        })),
        app_id: Some("gpuishell-dock".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "dock".to_string(),
            layer: Layer::Top,
            anchor,
            exclusive_zone: None,
            margin: Some((
                px(spacing::SM),
                px(spacing::SM),
                px(spacing::SM),
                px(spacing::SM),
            )),
            keyboard_interactivity: KeyboardInteractivity::None,
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Open a dock window on the given display, recording its display identity
/// for monitor-scoped item selection.
fn open_with_config(display_id: Option<DisplayId>, cx: &mut App) -> bool {
    match cx.open_window(window_options(display_id, cx), move |window, cx| {
        record_window_display(window.window_handle(), display_id);
        cx.new(Dock::new)
    }) {
        Ok(_) => true,
        Err(err) => {
            tracing::warn!("Failed to open dock window: {err}");
            false
        }
    }
}

fn open_all_dock_windows(cx: &mut App) -> usize {
    let mut opened = 0;
    let monitors = cx.config().dock.monitors;
    let displays = cx.displays();

    match monitors {
        DockMonitors::PrimaryOnly => {
            if open_with_config(cx.primary_display().map(|display| display.id()), cx) {
                opened += 1;
            }
        }
        DockMonitors::All if displays.is_empty() => {
            if open_with_config(None, cx) {
                opened += 1;
            }
        }
        DockMonitors::All => {
            for display in displays {
                if open_with_config(Some(display.id()), cx) {
                    opened += 1;
                }
            }
        }
    }

    opened
}

/// Rebuild all dock windows using the latest config.
pub fn reload(cx: &mut App) {
    let old_windows: Vec<_> = cx
        .windows()
        .into_iter()
        .filter(|handle| handle.downcast::<Dock>().is_some())
        .collect();

    if open_all_dock_windows(cx) == 0 {
        tracing::warn!("Dock reload skipped closing old windows because no new dock window opened");
        return;
    }

    for handle in old_windows {
        let _ = handle.update(cx, |_, window, _| window.remove_window());
    }
}

/// Initialize the dock using the current global config.
pub fn init(cx: &mut App) {
    cx.observe_global::<Config>(|cx| {
        tracing::info!("Config changed; reloading dock windows");
        reload(cx);
    })
    .detach();

    cx.spawn(async move |cx| {
        cx.background_executor()
            .timer(std::time::Duration::from_millis(100))
            .await;

        tracing::info!("Dock opened");
        cx.update(open_all_dock_windows)
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use super::windows_for_monitor;

    fn window(id: &str, monitor: &str) -> services::Window {
        services::Window {
            id: id.to_string(),
            app_id: "test-app".to_string(),
            title: "Test window".to_string(),
            monitor: monitor.to_string(),
            workspace_id: 1,
            is_focused: false,
            is_minimized: false,
            geometry: None,
        }
    }

    #[test]
    fn windows_for_monitor_only_keeps_windows_on_that_monitor() {
        let windows = vec![window("1", "eDP-1"), window("2", "HDMI-A-1")];

        let scoped = windows_for_monitor(&windows, Some("HDMI-A-1"));

        assert_eq!(scoped.len(), 1);
        assert_eq!(scoped[0].id, "2");
    }

    #[test]
    fn windows_for_monitor_without_a_resolved_monitor_keeps_all_windows() {
        let windows = vec![window("1", "eDP-1"), window("2", "HDMI-A-1")];

        let scoped = windows_for_monitor(&windows, None);

        assert_eq!(scoped, windows);
    }
}

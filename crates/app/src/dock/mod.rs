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

fn dock_items(
    windows: &[services::Window],
    apps: &[services::Application],
    pinned: &[String],
    monitor_name: Option<&str>,
) -> Vec<DockItem> {
    let scoped_windows = windows_for_monitor(windows, monitor_name);
    build_dock_items(&scoped_windows, apps, pinned)
}

fn dock_item_count(
    windows: &[services::Window],
    apps: &[services::Application],
    pinned: &[String],
    monitor_name: Option<&str>,
) -> usize {
    dock_items(windows, apps, pinned, monitor_name).len().max(1)
}

fn dock_window_size(
    position: crate::bar::config::BarPosition,
    icon_size: f32,
    item_count: usize,
    hover_effect: config::DockHoverEffect,
) -> Size<gpui::Pixels> {
    let primary_extent = item_count.max(1) as f32 * (icon_size + spacing::SM) + spacing::SM;
    let cross_extent = icon_size + spacing::SM * 2.0;
    let magnify_reserve = match hover_effect {
        config::DockHoverEffect::Magnify | config::DockHoverEffect::MagnifyLift => icon_size * 0.3,
        _ => 0.0,
    };
    let glow_reserve = match hover_effect {
        config::DockHoverEffect::Glow => 4.0,
        _ => 0.0,
    };
    let lift_reserve = match hover_effect {
        config::DockHoverEffect::Lift | config::DockHoverEffect::MagnifyLift => 8.0,
        _ => 0.0,
    };

    let primary_extent = primary_extent
        + magnify_reserve
        + glow_reserve
        + if position.is_vertical() {
            lift_reserve
        } else {
            0.0
        };
    let cross_extent = cross_extent
        + magnify_reserve
        + glow_reserve
        + if position.is_vertical() {
            0.0
        } else {
            lift_reserve
        };

    if position.is_vertical() {
        Size::new(px(cross_extent), px(primary_extent))
    } else {
        Size::new(px(primary_extent), px(cross_extent))
    }
}

fn monitor_name_for_display(
    display_id: Option<DisplayId>,
    state: &services::CompositorState,
    cx: &App,
) -> Option<String> {
    let display_uuid = cx.find_display(display_id?)?.uuid().ok()?;

    state
        .monitors
        .iter()
        .find(|monitor| {
            uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_DNS, monitor.name.as_bytes()) == display_uuid
        })
        .map(|monitor| monitor.name.clone())
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
        monitor_name_for_display(display_id_for_window(window), &self.state, cx)
    }

    fn items(&self, window: &Window, cx: &App) -> Vec<DockItem> {
        let apps = AppState::applications(cx).all();
        dock_items(
            &self.state.windows,
            apps,
            &cx.config().dock.pinned,
            self.current_monitor_name(window, cx).as_deref(),
        )
    }

    fn render_item(&self, item: &DockItem, cx: &Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let config = &cx.config().dock;
        let icon_size = config.icon_size;
        let hover_effect = config.hover_effect;
        let accent = theme.accent.primary;

        let mut element = div()
            .id(item.key.clone())
            .size(px(icon_size))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(radius::MD));

        element = match hover_effect {
            config::DockHoverEffect::None => element,
            config::DockHoverEffect::Lift => element.hover(move |style| style.mb(px(8.0))),
            config::DockHoverEffect::Magnify => {
                element.hover(move |style| style.size(px(icon_size * 1.3)))
            }
            config::DockHoverEffect::Glow => {
                element.hover(move |style| style.border_2().border_color(accent))
            }
            config::DockHoverEffect::MagnifyLift => {
                element.hover(move |style| style.size(px(icon_size * 1.3)).mb(px(8.0)))
            }
        };

        element
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
        let target_size = dock_window_size(
            cx.config().dock.position,
            cx.config().dock.icon_size,
            items.len(),
            cx.config().dock.hover_effect,
        );
        if window.viewport_size() != target_size {
            window.resize(target_size);
        }
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
    let compositor_state = AppState::compositor(cx).get();
    let item_count = dock_item_count(
        &compositor_state.windows,
        AppState::applications(cx).all(),
        &config.pinned,
        monitor_name_for_display(display_id, &compositor_state, cx).as_deref(),
    );
    let window_size = dock_window_size(
        config.position,
        config.icon_size,
        item_count,
        config.hover_effect,
    );

    let anchor = if is_vertical {
        match config.position {
            crate::bar::config::BarPosition::Left => Anchor::LEFT,
            _ => Anchor::RIGHT,
        }
    } else {
        match config.position {
            crate::bar::config::BarPosition::Top => Anchor::TOP,
            _ => Anchor::BOTTOM,
        }
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
    use super::{DockHoverEffect, dock_item_count, dock_window_size, windows_for_monitor};
    use crate::bar::config::BarPosition;
    use gpui::{Size, px};
    use std::path::PathBuf;

    fn app(name: &str, desktop_file: &str) -> services::Application {
        services::Application {
            name: name.to_string(),
            exec: name.to_string(),
            icon: None,
            icon_path: None,
            description: None,
            desktop_file: PathBuf::from(format!("/usr/share/applications/{desktop_file}")),
            startup_wm_class: None,
        }
    }

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

    #[test]
    fn dock_item_count_includes_pinned_and_monitor_local_unpinned_items() {
        let windows = vec![window("1", "HDMI-A-1"), window("2", "eDP-1")];
        let apps = vec![app("Pinned", "pinned.desktop")];

        let count = dock_item_count(
            &windows,
            &apps,
            &["pinned.desktop".to_string()],
            Some("HDMI-A-1"),
        );

        assert_eq!(count, 2);
    }

    #[test]
    fn dock_window_size_matches_horizontal_item_extent() {
        let size = dock_window_size(BarPosition::Bottom, 40.0, 2, DockHoverEffect::None);

        assert_eq!(size, Size::new(px(104.0), px(56.0)));
    }

    #[test]
    fn dock_window_size_reserves_lift_margin_for_each_orientation() {
        let horizontal = dock_window_size(BarPosition::Bottom, 40.0, 2, DockHoverEffect::Lift);
        let vertical = dock_window_size(BarPosition::Left, 40.0, 2, DockHoverEffect::Lift);

        assert_eq!(horizontal, Size::new(px(104.0), px(64.0)));
        assert_eq!(vertical, Size::new(px(56.0), px(112.0)));
    }

    #[test]
    fn dock_window_size_reserves_magnify_lift_for_each_orientation() {
        let horizontal =
            dock_window_size(BarPosition::Bottom, 40.0, 2, DockHoverEffect::MagnifyLift);
        let vertical = dock_window_size(BarPosition::Left, 40.0, 2, DockHoverEffect::MagnifyLift);

        assert_eq!(horizontal, Size::new(px(116.0), px(76.0)));
        assert_eq!(vertical, Size::new(px(68.0), px(124.0)));
    }
}

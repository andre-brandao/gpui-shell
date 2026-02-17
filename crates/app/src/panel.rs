//! Panel system for popup menus and overlays.
//!
//! Panels are layer shell windows that appear on top of other content,
//! typically used for dropdown menus, context menus, and popup dialogs.

use gpui::{
    layer_shell::*, prelude::*, px, AnyWindowHandle, App, Bounds, Point, Render, Size,
    WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions,
};
use std::sync::Mutex;

use crate::bar::modules::WidgetSlot;
use crate::config::BarPosition;

/// Panel configuration for positioning and sizing.
#[derive(Clone)]
pub struct PanelConfig {
    pub width: f32,
    pub height: f32,
    pub anchor: Anchor,
    pub margin: (f32, f32, f32, f32), // top, right, bottom, left
    pub namespace: String,
}

impl Default for PanelConfig {
    fn default() -> Self {
        PanelConfig {
            width: 320.0,
            height: 400.0,
            anchor: Anchor::TOP | Anchor::RIGHT,
            margin: (0.0, 8.0, 0.0, 0.0),
            namespace: "panel".to_string(),
        }
    }
}

/// Resolve panel anchor/margin for a bar position and widget slot.
pub fn panel_placement(
    bar_position: BarPosition,
    slot: WidgetSlot,
) -> (Anchor, (f32, f32, f32, f32)) {
    match bar_position {
        BarPosition::Left => {
            let vertical_edge = if matches!(slot, WidgetSlot::End) {
                Anchor::BOTTOM
            } else {
                Anchor::TOP
            };
            (Anchor::LEFT | vertical_edge, (0.0, 0.0, 0.0, 0.0))
        }
        BarPosition::Right => {
            let vertical_edge = if matches!(slot, WidgetSlot::End) {
                Anchor::BOTTOM
            } else {
                Anchor::TOP
            };
            (Anchor::RIGHT | vertical_edge, (0.0, 0.0, 0.0, 0.0))
        }
        BarPosition::Top => {
            let horizontal_edge = if matches!(slot, WidgetSlot::End) {
                Anchor::RIGHT
            } else {
                Anchor::LEFT
            };
            (Anchor::TOP | horizontal_edge, (0.0, 0.0, 0.0, 0.0))
        }
        BarPosition::Bottom => {
            let horizontal_edge = if matches!(slot, WidgetSlot::End) {
                Anchor::RIGHT
            } else {
                Anchor::LEFT
            };
            (Anchor::BOTTOM | horizontal_edge, (0.0, 0.0, 0.0, 0.0))
        }
    }
}

/// Global panel manager to ensure only one panel is open at a time.
static ACTIVE_PANEL: Mutex<Option<(String, AnyWindowHandle)>> = Mutex::new(None);

/// Open a panel with the given ID and content.
/// If a panel with the same ID is already open, it will be closed.
/// If a different panel is open, it will be closed first.
/// Returns true if the panel was opened, false if it was closed (toggled off).
pub fn toggle_panel<V: Render + 'static>(
    panel_id: &str,
    config: PanelConfig,
    cx: &mut App,
    build: impl FnOnce(&mut gpui::Context<V>) -> V + 'static,
) -> bool {
    let mut guard = ACTIVE_PANEL.lock().unwrap();

    // Check if any panel is open
    if let Some((open_id, handle)) = guard.take() {
        // Close the existing panel
        let _ = cx.update_window(handle, |_, window, _cx| {
            window.remove_window();
        });

        // If it was the same panel, just close it (toggle off)
        if open_id == panel_id {
            return false;
        }
    }

    // Open new panel
    let window_options = WindowOptions {
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: Point::new(px(0.), px(0.)),
            size: Size::new(px(config.width), px(config.height)),
        })),
        app_id: Some(format!("gpuishell-panel-{}", panel_id)),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: config.namespace,
            layer: Layer::Overlay,
            anchor: config.anchor,
            exclusive_zone: None,
            margin: Some((
                px(config.margin.0),
                px(config.margin.1),
                px(config.margin.2),
                px(config.margin.3),
            )),
            keyboard_interactivity: KeyboardInteractivity::OnDemand,
            ..Default::default()
        }),
        focus: true,
        ..Default::default()
    };

    if let Ok(handle) = cx.open_window(window_options, move |_, cx| cx.new(build)) {
        *guard = Some((panel_id.to_string(), handle.into()));
        true
    } else {
        false
    }
}

/// Close any open panel.
#[allow(dead_code)]
pub fn close_panel(cx: &mut App) {
    let mut guard = ACTIVE_PANEL.lock().unwrap();
    if let Some((_, handle)) = guard.take() {
        let _ = cx.update_window(handle, |_, window, _cx| {
            window.remove_window();
        });
    }
}

/// Check if a specific panel is open.
#[allow(dead_code)]
pub fn is_panel_open(panel_id: &str) -> bool {
    ACTIVE_PANEL
        .lock()
        .map(|guard| {
            guard
                .as_ref()
                .map(|(id, _)| id == panel_id)
                .unwrap_or(false)
        })
        .unwrap_or(false)
}

/// Get the ID of the currently open panel, if any.
#[allow(dead_code)]
pub fn active_panel_id() -> Option<String> {
    ACTIVE_PANEL
        .lock()
        .ok()
        .and_then(|guard| guard.as_ref().map(|(id, _)| id.clone()))
}

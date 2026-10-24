use gpui::{
    App, Bounds, DisplayId, WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions,
    layer_shell::*, point, prelude::*, px,
};
use ui::Spacing;

use futures::StreamExt;
use futures_signals::signal::SignalExt;

use crate::config::{ActiveConfig, ConfigSignals, State};
use crate::state::{AppState, record_window_display};

use super::{Dock, DockMonitors, dock_item_count, dock_window_size, monitor_name_for_display};

fn window_options(display_id: Option<DisplayId>, cx: &App) -> WindowOptions {
    let config = &cx.config().dock;
    let is_vertical = config.position.is_vertical();
    let compositor_state = AppState::compositor(cx).get();
    let item_count = dock_item_count(
        &compositor_state.windows,
        AppState::applications(cx).all(),
        State::pinned(cx),
        monitor_name_for_display(display_id, &compositor_state, cx).as_deref(),
    ) + 1;
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
                Spacing::Medium.pixels(),
                Spacing::Medium.pixels(),
                Spacing::Medium.pixels(),
                Spacing::Medium.pixels(),
            )),
            keyboard_interactivity: KeyboardInteractivity::None,
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn open_with_config(display_id: Option<DisplayId>, cx: &mut App) -> bool {
    match cx.open_window(window_options(display_id, cx), move |window, cx| {
        record_window_display(window.window_handle(), display_id, cx);
        cx.new(Dock::new)
    }) {
        Ok(_) => true,
        Err(err) => {
            tracing::warn!("Failed to open dock window: {err}");
            false
        }
    }
}

fn open_all(cx: &mut App) -> usize {
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

pub fn reload(cx: &mut App) {
    let old_windows: Vec<_> = cx
        .windows()
        .into_iter()
        .filter(|handle| handle.downcast::<Dock>().is_some())
        .collect();

    if open_all(cx) == 0 {
        tracing::warn!("Dock reload skipped closing old windows because no new dock window opened");
        return;
    }

    for handle in old_windows {
        let _ = handle.update(cx, |_, window, _| window.remove_window());
    }
}

pub fn init(cx: &mut App) {
    // Only the layer-shell anchor and the monitor set need new windows; size,
    // pins and the rest are applied by `Dock::render`.
    let layout = ConfigSignals::global(cx)
        .dock
        .signal_ref(|dock| (dock.position, dock.monitors))
        .dedupe()
        .to_stream();

    cx.spawn(async move |cx| {
        cx.background_executor()
            .timer(std::time::Duration::from_millis(100))
            .await;

        let mut layout = layout.skip(1);
        tracing::info!("Dock opened");
        cx.update(open_all);
        while layout.next().await.is_some() {
            tracing::info!("Dock layout changed; reopening dock windows");
            cx.update(reload);
        }
    })
    .detach();
}

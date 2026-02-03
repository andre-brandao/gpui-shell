//! On-Screen Display (OSD) for volume and brightness changes.
//!
//! Shows a brief overlay with icon, progress bar, and percentage
//! when volume or brightness changes. Auto-dismisses after 2 seconds.

use std::sync::Mutex;
use std::time::Duration;

use futures_signals::signal::SignalExt;
use gpui::{
    AnyWindowHandle, App, Bounds, Context, Entity, Point, Render, Size, Window,
    WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions, div, layer_shell::*,
    prelude::*, px,
};
use services::Services;
use ui::{accent, bg, border, icon_size, radius, spacing, text};

use crate::control_center::icons;

const OSD_WIDTH: f32 = 280.0;
const OSD_HEIGHT: f32 = 56.0;
const OSD_TIMEOUT: Duration = Duration::from_secs(2);

/// What the OSD is currently displaying.
#[derive(Debug, Clone, Copy, PartialEq)]
enum OsdKind {
    Volume { level: u8, muted: bool },
    Brightness { level: u8 },
}

/// The OSD view rendered inside the layer-shell window.
struct OsdView {
    kind: OsdKind,
}

impl OsdView {
    fn new(kind: OsdKind) -> Self {
        Self { kind }
    }
}

impl Render for OsdView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let (icon, level, muted) = match self.kind {
            OsdKind::Volume { level, muted } => (icons::volume_icon(level, muted), level, muted),
            OsdKind::Brightness { level } => {
                let icon = if level < 33 {
                    icons::BRIGHTNESS_LOW
                } else if level < 66 {
                    icons::BRIGHTNESS
                } else {
                    icons::BRIGHTNESS_HIGH
                };
                (icon, level, false)
            }
        };

        let fill_color = if muted {
            ui::status::error()
        } else {
            accent::primary()
        };

        // Calculate fill width as pixels based on available bar space
        // Total inner width: OSD_WIDTH - 16 (outer padding) - 2*MD padding - icon(~22) - gap - label(36) - gap
        // Approximate bar width is ~160px, but flex_1 handles it — use percentage of a known max
        let bar_fill_pct = level as f32 / 100.0;

        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .w(px(OSD_WIDTH - 16.0))
                    .h(px(OSD_HEIGHT - 16.0))
                    .px(px(spacing::MD))
                    .bg(bg::primary())
                    .border_1()
                    .border_color(border::default())
                    .rounded(px(radius::LG))
                    .flex()
                    .items_center()
                    .gap(px(spacing::MD))
                    // Icon
                    .child(
                        div()
                            .text_size(px(icon_size::XL))
                            .text_color(if muted {
                                ui::status::error()
                            } else {
                                text::primary()
                            })
                            .child(icon),
                    )
                    // Progress bar
                    .child(
                        div()
                            .flex_1()
                            .h(px(6.0))
                            .bg(bg::tertiary())
                            .rounded(px(3.0))
                            .overflow_hidden()
                            .child(
                                div()
                                    .h_full()
                                    .w(gpui::relative(bar_fill_pct))
                                    .bg(fill_color)
                                    .rounded(px(3.0)),
                            ),
                    )
                    // Percentage
                    .child(
                        div()
                            .w(px(36.0))
                            .text_size(px(12.0))
                            .text_color(text::secondary())
                            .text_right()
                            .child(format!("{}%", level)),
                    ),
            )
    }
}

/// Global OSD state.
static OSD_STATE: Mutex<Option<OsdWindowState>> = Mutex::new(None);

struct OsdWindowState {
    handle: AnyWindowHandle,
    view: Entity<OsdView>,
}

fn window_options() -> WindowOptions {
    WindowOptions {
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: Point::new(px(0.), px(0.)),
            size: Size::new(px(OSD_WIDTH), px(OSD_HEIGHT)),
        })),
        app_id: Some("gpuishell-osd".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "osd".to_string(),
            layer: Layer::Overlay,
            anchor: Anchor::BOTTOM,
            exclusive_zone: None,
            margin: Some((px(0.), px(0.), px(48.), px(0.))),
            keyboard_interactivity: KeyboardInteractivity::None,
            ..Default::default()
        }),
        focus: false,
        ..Default::default()
    }
}

/// Show or update the OSD with new content, resetting the dismiss timer.
fn show_osd(kind: OsdKind, cx: &mut App) {
    let mut guard = OSD_STATE.lock().unwrap();

    // If OSD window already exists, update it
    if let Some(state) = guard.as_ref() {
        let view = state.view.clone();
        let handle = state.handle;
        let ok = cx
            .update_window(handle, |_, _, cx| {
                view.update(cx, |osd, cx| {
                    osd.kind = kind;
                    cx.notify();
                });
            })
            .is_ok();

        if ok {
            // Reset dismiss timer
            schedule_dismiss(cx);
            return;
        }
        // Window was closed externally, fall through to create new one
    }

    // Create new OSD window
    let result = cx.open_window(window_options(), move |_, cx| {
        cx.new(|_| OsdView::new(kind))
    });

    if let Ok(handle) = result {
        let view = handle.update(cx, |_, _, cx| cx.entity().clone()).unwrap();
        *guard = Some(OsdWindowState {
            handle: handle.into(),
            view,
        });
        schedule_dismiss(cx);
    }
}

/// Close the OSD window.
fn close_osd(cx: &mut App) {
    let mut guard = OSD_STATE.lock().unwrap();
    if let Some(state) = guard.take() {
        let _ = cx.update_window(state.handle, |_, window, _cx| {
            window.remove_window();
        });
    }
}

/// Generation counter for dismiss scheduling.
static DISMISS_GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Schedule the OSD to be dismissed after the timeout.
/// Each call increments the generation so previous timers become stale.
fn schedule_dismiss(cx: &mut App) {
    let generation = DISMISS_GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;

    cx.spawn(async move |cx| {
        tokio::time::sleep(OSD_TIMEOUT).await;

        // Only dismiss if no newer show_osd call happened
        let current = DISMISS_GENERATION.load(std::sync::atomic::Ordering::SeqCst);
        if generation == current {
            let _ = cx.update(|cx| close_osd(cx));
        }
    })
    .detach();
}

/// Start listening for audio and brightness changes and show the OSD.
///
/// Should be called once during app initialization.
pub fn start(services: Services, cx: &mut App) {
    // Track initial values to only show OSD on changes (not on startup)
    let initial_audio = services.audio.get();
    let initial_brightness = services.brightness.get();

    // Audio listener
    cx.spawn({
        let mut signal = services.audio.subscribe().to_stream();
        let audio = services.audio.clone();
        let mut prev_volume = initial_audio.sink_volume;
        let mut prev_muted = initial_audio.sink_muted;

        async move |cx| {
            use futures_util::StreamExt;
            // Skip the initial value emitted by the signal
            signal.next().await;

            while signal.next().await.is_some() {
                let data = audio.get();
                if data.sink_volume != prev_volume || data.sink_muted != prev_muted {
                    prev_volume = data.sink_volume;
                    prev_muted = data.sink_muted;
                    let kind = OsdKind::Volume {
                        level: data.sink_volume,
                        muted: data.sink_muted,
                    };
                    cx.update(|cx| show_osd(kind, cx));
                }
            }
        }
    })
    .detach();

    // Brightness listener
    cx.spawn({
        let mut signal = services.brightness.subscribe().to_stream();
        let brightness = services.brightness.clone();
        let mut prev_percent = initial_brightness.percentage();

        async move |cx| {
            use futures_util::StreamExt;
            // Skip the initial value
            signal.next().await;

            while signal.next().await.is_some() {
                let data = brightness.get();
                let percent = data.percentage();
                if percent != prev_percent {
                    prev_percent = percent;
                    let kind = OsdKind::Brightness { level: percent };
                    cx.update(|cx| show_osd(kind, cx));
                }
            }
        }
    })
    .detach();
}

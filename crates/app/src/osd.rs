//! On-Screen Display (OSD) for volume and brightness changes.
//!
//! Shows a brief overlay with icon, progress bar, and percentage
//! when volume or brightness changes. Auto-dismisses after 2 seconds.
//!
//! Supports four positions: Top, Bottom, Left, Right.
//! Left/Right use a vertical layout; Top/Bottom use a horizontal layout.

use std::sync::Mutex;
use std::time::Duration;

use futures_signals::signal::SignalExt;
use gpui::{
    AnyWindowHandle, App, Bounds, Context, Entity, Point, Render, Size, Window,
    WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions, div, layer_shell::*,
    prelude::*, px,
};
use services::Services;
use ui::{ActiveTheme, icon_size, radius, spacing};

use crate::control_center::icons;

const OSD_LONG: f32 = 280.0;
const OSD_SHORT: f32 = 56.0;
const OSD_MARGIN: f32 = 24.0;
const OSD_TIMEOUT: Duration = Duration::from_secs(2);

/// OSD screen position.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum OsdPosition {
    Top,
    Bottom,
    Left,
    #[default]
    Right,
}

impl OsdPosition {
    fn is_vertical(self) -> bool {
        matches!(self, OsdPosition::Left | OsdPosition::Right)
    }

    fn window_size(self) -> (f32, f32) {
        if self.is_vertical() {
            (OSD_SHORT, OSD_LONG)
        } else {
            (OSD_LONG, OSD_SHORT)
        }
    }

    fn anchor(self) -> Anchor {
        match self {
            OsdPosition::Top => Anchor::TOP,
            OsdPosition::Bottom => Anchor::BOTTOM,
            OsdPosition::Left => Anchor::LEFT,
            OsdPosition::Right => Anchor::RIGHT,
        }
    }

    fn margin(self) -> (f32, f32, f32, f32) {
        match self {
            OsdPosition::Top => (OSD_MARGIN, 0., 0., 0.),
            OsdPosition::Bottom => (0., 0., OSD_MARGIN, 0.),
            OsdPosition::Left => (0., 0., 0., OSD_MARGIN),
            OsdPosition::Right => (0., OSD_MARGIN, 0., 0.),
        }
    }
}

/// What the OSD is currently displaying.
#[derive(Debug, Clone, Copy, PartialEq)]
enum OsdKind {
    Volume { level: u8, muted: bool },
    Brightness { level: u8 },
}

/// The OSD view rendered inside the layer-shell window.
struct OsdView {
    kind: OsdKind,
    position: OsdPosition,
}

impl OsdView {
    fn new(kind: OsdKind, position: OsdPosition) -> Self {
        Self { kind, position }
    }

    fn icon_and_level(&self) -> (&'static str, u8, bool) {
        match self.kind {
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
        }
    }

    fn render_horizontal(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let (icon, level, muted) = self.icon_and_level();

        let fill_color = if muted {
            theme.status.error
        } else if level > 100 {
            theme.status.warning
        } else {
            theme.accent.primary
        };

        let bar_fill_pct = (level as f32 / 100.0).min(1.0);

        let icon_color = if muted {
            theme.status.error
        } else {
            theme.text.primary
        };

        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .w(px(OSD_LONG - 16.0))
                    .h(px(OSD_SHORT - 16.0))
                    .px(px(spacing::MD))
                    .bg(theme.bg.primary)
                    .border_1()
                    .border_color(theme.border.default)
                    .rounded(px(radius::LG))
                    .flex()
                    .items_center()
                    .gap(px(spacing::MD))
                    .child(
                        div()
                            .text_size(px(icon_size::XL))
                            .text_color(icon_color)
                            .child(icon),
                    )
                    .child(
                        div()
                            .flex_1()
                            .h(px(6.0))
                            .bg(theme.bg.tertiary)
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
                    .child(
                        div()
                            .w(px(36.0))
                            .text_size(px(12.0))
                            .text_color(theme.text.secondary)
                            .text_right()
                            .child(format!("{}%", level)),
                    ),
            )
    }

    fn render_vertical(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let (icon, level, muted) = self.icon_and_level();

        let fill_color = if muted {
            theme.status.error
        } else if level > 100 {
            theme.status.warning
        } else {
            theme.accent.primary
        };

        let bar_fill_pct = (level as f32 / 100.0).min(1.0);

        let icon_color = if muted {
            theme.status.error
        } else {
            theme.text.primary
        };

        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .w(px(OSD_SHORT - 16.0))
                    .h(px(OSD_LONG - 16.0))
                    .py(px(spacing::MD))
                    .bg(theme.bg.primary)
                    .border_1()
                    .border_color(theme.border.default)
                    .rounded(px(radius::LG))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(spacing::MD))
                    // Percentage at top
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(theme.text.secondary)
                            .child(format!("{}%", level)),
                    )
                    // Vertical progress bar (grows upward from bottom)
                    .child(
                        div()
                            .flex_1()
                            .w(px(6.0))
                            .bg(theme.bg.tertiary)
                            .rounded(px(3.0))
                            .overflow_hidden()
                            .flex()
                            .flex_col()
                            .justify_end()
                            .child(
                                div()
                                    .w_full()
                                    .h(gpui::relative(bar_fill_pct))
                                    .bg(fill_color)
                                    .rounded(px(3.0)),
                            ),
                    )
                    // Icon at bottom
                    .child(
                        div()
                            .text_size(px(icon_size::XL))
                            .text_color(icon_color)
                            .child(icon),
                    ),
            )
    }
}

impl Render for OsdView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.position.is_vertical() {
            self.render_vertical(cx).into_any_element()
        } else {
            self.render_horizontal(cx).into_any_element()
        }
    }
}

/// Global OSD state.
static OSD_STATE: Mutex<Option<OsdWindowState>> = Mutex::new(None);
static OSD_POSITION: Mutex<OsdPosition> = Mutex::new(OsdPosition::Right);

struct OsdWindowState {
    handle: AnyWindowHandle,
    view: Entity<OsdView>,
}

fn window_options(position: OsdPosition) -> WindowOptions {
    let (w, h) = position.window_size();
    let margin = position.margin();

    WindowOptions {
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: Point::new(px(0.), px(0.)),
            size: Size::new(px(w), px(h)),
        })),
        app_id: Some("gpuishell-osd".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "osd".to_string(),
            layer: Layer::Overlay,
            anchor: position.anchor(),
            exclusive_zone: None,
            margin: Some((px(margin.0), px(margin.1), px(margin.2), px(margin.3))),
            keyboard_interactivity: KeyboardInteractivity::None,
            ..Default::default()
        }),
        focus: false,
        ..Default::default()
    }
}

/// Show or update the OSD with new content, resetting the dismiss timer.
fn show_osd(kind: OsdKind, cx: &mut App) {
    let position = *OSD_POSITION.lock().unwrap();
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
            schedule_dismiss(cx);
            return;
        }
        // Window was closed externally, fall through to create new one
    }

    // Create new OSD window
    let result = cx.open_window(window_options(position), move |_, cx| {
        cx.new(|_| OsdView::new(kind, position))
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
pub fn start(services: Services, position: OsdPosition, cx: &mut App) {
    *OSD_POSITION.lock().unwrap() = position;

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

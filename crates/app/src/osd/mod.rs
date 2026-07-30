//! On-Screen Display (OSD) for volume and brightness changes.
//!
//! Shows a brief overlay with icon, progress bar, and percentage
//! when volume or brightness changes. Auto-dismisses after 2 seconds.
//!
//! Supports four positions: Top, Bottom, Left, Right.
//! Left/Right use a vertical layout; Top/Bottom use a horizontal layout.

mod config;

pub use config::{OsdConfig, OsdPosition};

use std::sync::Mutex;
use std::time::Duration;

use futures_signals::signal::SignalExt;
use gpui::{
    AnyWindowHandle, App, Bounds, Context, Entity, Point, Render, Size, Window,
    WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions, div, layer_shell::*,
    prelude::*, px,
};
use ui::patterns::OsdIndicator;
use ui::{ActiveTheme, IconName};

use crate::config::Config;
use crate::icons;
use crate::state::AppState;

const OSD_LONG: f32 = 280.0;
const OSD_SHORT: f32 = 56.0;
const OSD_MARGIN: f32 = 24.0;
const OSD_TIMEOUT: Duration = Duration::from_secs(2);

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

    fn icon_and_level(&self) -> (ui::IconName, u8, bool) {
        match self.kind {
            OsdKind::Volume { level, muted } => (icons::volume_icon(level, muted), level, muted),
            OsdKind::Brightness { level } => (IconName::Sun, level, false),
        }
    }
}

impl Render for OsdView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.set_rem_size(cx.theme().font_size);

        let theme = cx.theme();
        let (icon, level, muted) = self.icon_and_level();

        // What the colour says is ours to decide; the pill just paints it.
        let fill = if muted {
            theme.colors.status.error
        } else if level > 100 {
            theme.colors.status.warning
        } else {
            theme.colors.accent
        };
        let icon_color = if muted {
            theme.colors.status.error
        } else {
            theme.colors.text
        };

        // 8px inset all round is what the window's long/short extent leaves
        // the pill.
        div().size_full().p(px(8.0)).child(
            OsdIndicator::new(icon, level)
                .vertical(self.position.is_vertical())
                .fill(fill)
                .icon_color(icon_color),
        )
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
        cx.background_executor().timer(OSD_TIMEOUT).await;

        // Only dismiss if no newer show_osd call happened
        let current = DISMISS_GENERATION.load(std::sync::atomic::Ordering::SeqCst);
        if generation == current {
            cx.update(close_osd);
        }
    })
    .detach();
}

/// Initialize OSD listeners for audio and brightness changes.
///
/// Should be called once during app initialization.
pub fn init(cx: &mut App) {
    let audio_service = AppState::audio(cx).clone();
    let brightness_service = AppState::brightness(cx).clone();
    let position = Config::global(cx).osd.position;
    *OSD_POSITION.lock().unwrap() = position;

    // Track initial values to only show OSD on changes (not on startup)
    let initial_audio = audio_service.get();
    let initial_brightness = brightness_service.get();

    // Audio listener
    cx.spawn({
        let mut signal = audio_service.subscribe().to_stream();
        let audio = audio_service.clone();
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
        let mut signal = brightness_service.subscribe().to_stream();
        let brightness = brightness_service.clone();
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

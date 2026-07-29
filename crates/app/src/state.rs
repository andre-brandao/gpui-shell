//! Application-wide runtime state stored as a GPUI global.

use std::collections::HashMap;
use std::future::Future;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use anyhow::Context as _;
use futures_signals::signal::{Signal, SignalExt};
use futures_util::StreamExt;
use gpui::{AnyWindowHandle, App, Context, DisplayId, Global, Window};

/// Maps each bar window to the display it was explicitly opened on.
///
/// We can't correlate a window to a compositor monitor by position: gpui's
/// `Display::bounds()` doesn't reflect real global monitor placement in this
/// fork (every display reports origin (0, 0) regardless of its actual
/// position), and `Window::display()` is separately unreliable for
/// layer-shell windows (gpui_linux derives it from `primary_output_scale()`,
/// which just picks whichever output has the highest scale factor). We also
/// can't assume `cx.displays()` and the compositor's own monitor list
/// enumerate outputs in the same order - on at least one real dual-monitor
/// setup they didn't, which silently swapped which bar controlled which
/// monitor. So instead we record the specific `DisplayId` each window was
/// opened with, and match it against a compositor monitor by name via
/// `PlatformDisplay::uuid()` (see the `bar` and `dock` modules'
/// monitor-matching code), which gpui_linux derives deterministically from
/// the Wayland output's name.
static WINDOW_DISPLAYS: Mutex<Option<HashMap<AnyWindowHandle, DisplayId>>> = Mutex::new(None);

/// Look up the display a bar window was opened on.
pub fn display_id_for_window(window: &Window) -> Option<DisplayId> {
    WINDOW_DISPLAYS
        .lock()
        .ok()?
        .as_ref()?
        .get(&window.window_handle())
        .copied()
}

/// Find the compositor monitor backing a gpui display.
///
/// `gpui_linux`'s `PlatformDisplay::uuid()` is a deterministic hash of the
/// Wayland output's name (`Uuid::new_v5(NAMESPACE_DNS, name)`), so hashing
/// each compositor monitor name the same way identifies the pair without
/// relying on enumeration order - which `cx.displays()` and the
/// compositor's own list do not share (see the note on [`WINDOW_DISPLAYS`]).
pub(crate) fn monitor_for_display<'a>(
    display_id: Option<DisplayId>,
    state: &'a services::CompositorState,
    cx: &App,
) -> Option<&'a services::Monitor> {
    let display_uuid = cx.find_display(display_id?)?.uuid().ok()?;
    state.monitors.iter().find(|monitor| {
        uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_DNS, monitor.name.as_bytes()) == display_uuid
    })
}

/// Logical (scale-corrected) size of the monitor backing a gpui display.
///
/// Prefer this over `PlatformDisplay::bounds()` for anything that has to
/// match the real screen extent. gpui derives its display bounds from the
/// integer `wl_output.scale`, and Wayland's core output protocol carries no
/// fractional scale - a screen running at 1.5 advertises 2, so gpui reports
/// two thirds of the true logical size.
///
/// Returns `None` when the compositor backend does not publish monitor
/// geometry (the Niri backend leaves it at zero), leaving the caller to
/// fall back to gpui's own bounds.
pub(crate) fn logical_display_size(
    display_id: Option<DisplayId>,
    cx: &App,
) -> Option<gpui::Size<gpui::Pixels>> {
    let state = AppState::compositor(cx).get();
    let monitor = monitor_for_display(display_id, &state, cx)?;
    if monitor.scale <= 0.0 || monitor.width == 0 || monitor.height == 0 {
        return None;
    }
    Some(gpui::size(
        gpui::px(monitor.width as f32 / monitor.scale),
        gpui::px(monitor.height as f32 / monitor.scale),
    ))
}

pub(crate) fn record_window_display(handle: AnyWindowHandle, display_id: Option<DisplayId>) {
    let Some(display_id) = display_id else {
        return;
    };
    let mut guard = WINDOW_DISPLAYS.lock().unwrap();
    guard
        .get_or_insert_with(HashMap::new)
        .insert(handle, display_id);
}

/// Shared services container for all system integrations.
///
/// This struct holds instances of all available services and should be
/// initialized once at application startup, then shared with widgets
/// that need access to system information.
#[derive(Clone)]
pub(crate) struct Services {
    pub applications: services::ApplicationsService,
    pub audio: services::AudioSubscriber,
    pub bluetooth: services::BluetoothSubscriber,
    pub brightness: services::BrightnessSubscriber,
    pub compositor: services::CompositorSubscriber,
    pub mpris: services::MprisSubscriber,
    pub network: services::NetworkSubscriber,
    pub notification: services::NotificationSubscriber,
    pub privacy: services::PrivacySubscriber,
    pub sysinfo: services::SysInfoSubscriber,
    pub tray: services::TraySubscriber,
    pub upower: services::UPowerSubscriber,
    pub wallpaper: services::WallpaperSubscriber,
}

const SERVICE_INIT_TIMEOUT: Duration = Duration::from_secs(5);

fn init_sync_service<T>(name: &'static str, init: impl FnOnce() -> T) -> T {
    let started = Instant::now();
    tracing::info!("Initializing {name} service");
    let service = init();
    tracing::info!("Initialized {name} service in {:?}", started.elapsed());
    service
}

async fn init_async_service<T, F>(name: &'static str, future: F) -> anyhow::Result<T>
where
    F: Future<Output = anyhow::Result<T>>,
{
    let started = Instant::now();
    tracing::info!("Initializing {name} service");

    let service = tokio::time::timeout(SERVICE_INIT_TIMEOUT, future)
        .await
        .with_context(|| format!("Timed out initializing {name} service"))??;

    tracing::info!("Initialized {name} service in {:?}", started.elapsed());
    Ok(service)
}

async fn init_optional_service<T, F, G>(
    name: &'static str,
    future: F,
    fallback: G,
) -> anyhow::Result<T>
where
    F: Future<Output = anyhow::Result<T>>,
    G: Future<Output = anyhow::Result<T>>,
{
    match init_async_service(name, future).await {
        Ok(service) => Ok(service),
        Err(err) => {
            tracing::warn!("{name} service unavailable during startup: {err:#}");
            fallback
                .await
                .with_context(|| format!("Failed to initialize fallback for {name} service"))
        }
    }
}

pub(crate) async fn init_services() -> anyhow::Result<Services> {
    let applications = init_sync_service("applications", services::ApplicationsService::new);
    let audio = init_sync_service("audio", services::AudioSubscriber::new);
    let bluetooth = init_optional_service(
        "bluetooth",
        services::BluetoothSubscriber::new(),
        services::BluetoothSubscriber::unavailable(),
    )
    .await?;
    let brightness =
        init_async_service("brightness", services::BrightnessSubscriber::new()).await?;
    let compositor =
        init_async_service("compositor", services::CompositorSubscriber::new()).await?;
    let mpris = init_optional_service(
        "mpris",
        services::MprisSubscriber::new(),
        services::MprisSubscriber::unavailable(),
    )
    .await?;
    let network = init_optional_service(
        "network",
        services::NetworkSubscriber::new(),
        services::NetworkSubscriber::unavailable(),
    )
    .await?;
    let notification = init_async_service("notification", services::NotificationSubscriber::new())
        .await
        .unwrap_or_else(|err| {
            tracing::warn!("Notification service unavailable during startup: {err:#}");
            services::NotificationSubscriber::disabled()
        });
    let privacy = init_sync_service("privacy", services::PrivacySubscriber::new);
    let sysinfo = init_sync_service("sysinfo", services::SysInfoSubscriber::new);
    let tray = init_optional_service(
        "tray",
        services::TraySubscriber::new(),
        services::TraySubscriber::unavailable(),
    )
    .await?;
    let upower = init_optional_service(
        "upower",
        services::UPowerSubscriber::new(),
        services::UPowerSubscriber::unavailable(),
    )
    .await?;
    let wallpaper = init_sync_service("wallpaper", services::WallpaperSubscriber::new);

    Ok(Services {
        applications,
        audio,
        bluetooth,
        brightness,
        compositor,
        mpris,
        network,
        notification,
        privacy,
        sysinfo,
        tray,
        upower,
        wallpaper,
    })
}

/// Watch a signal and apply updates to component state.
pub(crate) fn watch<C, S, T, F>(cx: &mut Context<C>, signal: S, on_update: F)
where
    C: 'static,
    S: Signal<Item = T> + Unpin + 'static,
    T: Clone + 'static,
    F: Fn(&mut C, T, &mut Context<C>) + 'static,
{
    cx.spawn(async move |this, cx| {
        let mut stream = signal.to_stream();
        while let Some(data) = stream.next().await {
            if this
                .update(cx, |this, cx| {
                    on_update(this, data.clone(), cx);
                })
                .is_err()
            {
                break;
            }
        }
    })
    .detach();
}

/// Global runtime state shared across views/widgets.
#[derive(Clone)]
pub struct AppState {
    services: Services,
}

impl Global for AppState {}

impl AppState {
    /// Initialize the global app state.
    pub(crate) fn init(services: Services, cx: &mut App) {
        cx.set_global(Self { services });
    }

    /// Get the global app state.
    #[inline(always)]
    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    #[inline(always)]
    fn services(cx: &App) -> &Services {
        &Self::global(cx).services
    }

    #[inline(always)]
    pub fn applications(cx: &App) -> &services::ApplicationsService {
        &Self::services(cx).applications
    }

    #[inline(always)]
    pub fn audio(cx: &App) -> &services::AudioSubscriber {
        &Self::services(cx).audio
    }

    #[inline(always)]
    pub fn bluetooth(cx: &App) -> &services::BluetoothSubscriber {
        &Self::services(cx).bluetooth
    }

    #[inline(always)]
    pub fn brightness(cx: &App) -> &services::BrightnessSubscriber {
        &Self::services(cx).brightness
    }

    #[inline(always)]
    pub fn compositor(cx: &App) -> &services::CompositorSubscriber {
        &Self::services(cx).compositor
    }

    #[inline(always)]
    pub fn mpris(cx: &App) -> &services::MprisSubscriber {
        &Self::services(cx).mpris
    }

    #[inline(always)]
    pub fn network(cx: &App) -> &services::NetworkSubscriber {
        &Self::services(cx).network
    }

    #[inline(always)]
    pub fn notification(cx: &App) -> &services::NotificationSubscriber {
        &Self::services(cx).notification
    }

    #[inline(always)]
    pub fn privacy(cx: &App) -> &services::PrivacySubscriber {
        &Self::services(cx).privacy
    }

    #[inline(always)]
    pub fn sysinfo(cx: &App) -> &services::SysInfoSubscriber {
        &Self::services(cx).sysinfo
    }

    #[inline(always)]
    pub fn tray(cx: &App) -> &services::TraySubscriber {
        &Self::services(cx).tray
    }

    #[inline(always)]
    pub fn upower(cx: &App) -> &services::UPowerSubscriber {
        &Self::services(cx).upower
    }

    #[inline(always)]
    pub fn wallpaper(cx: &App) -> &services::WallpaperSubscriber {
        &Self::services(cx).wallpaper
    }
}

//! Application-wide runtime state stored as a GPUI global.

use futures_signals::signal::{Signal, SignalExt};
use futures_util::StreamExt;
use gpui::{App, Context, Global};

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

pub(crate) async fn init_services() -> anyhow::Result<Services> {
    let applications = services::ApplicationsService::new();
    let audio = services::AudioSubscriber::new();
    let bluetooth = services::BluetoothSubscriber::new().await?;
    let brightness = services::BrightnessSubscriber::new().await?;
    let compositor = services::CompositorSubscriber::new().await?;
    let mpris = services::MprisSubscriber::new().await?;
    let network = services::NetworkSubscriber::new().await?;
    let notification = services::NotificationSubscriber::new().await.unwrap_or_else(|err| {
        tracing::warn!("Notification service unavailable: {}", err);
        services::NotificationSubscriber::disabled()
    });
    let privacy = services::PrivacySubscriber::new();
    let sysinfo = services::SysInfoSubscriber::new();
    let tray = services::TraySubscriber::new().await?;
    let upower = services::UPowerSubscriber::new().await?;
    let wallpaper = services::WallpaperSubscriber::new();

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

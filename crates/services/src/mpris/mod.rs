//! MPRIS media player service for playback state and controls.

mod dbus;

use std::collections::HashMap;
use std::fmt::Display;
use std::thread;

use dbus::MprisPlayerProxy;
use futures_signals::signal::{Mutable, MutableSignalCloned};
use futures_util::StreamExt;
use futures_util::future::join_all;
use futures_util::stream::select_all;
use tracing::{debug, error, info, trace, warn};
use zbus::{Connection, fdo::DBusProxy, zvariant::OwnedValue};

const MPRIS_PLAYER_SERVICE_PREFIX: &str = "org.mpris.MediaPlayer2.";
const EVENT_DEBOUNCE_MS: u64 = 200;

/// Current playback state reported by MPRIS.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackStatus {
    Playing,
    Paused,
    #[default]
    Stopped,
}

impl From<String> for PlaybackStatus {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

impl From<&str> for PlaybackStatus {
    fn from(value: &str) -> Self {
        match value {
            "Playing" => Self::Playing,
            "Paused" => Self::Paused,
            "Stopped" => Self::Stopped,
            _ => Self::Stopped,
        }
    }
}

/// Simplified MPRIS metadata for display.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MprisPlayerMetadata {
    pub artists: Option<Vec<String>>,
    pub title: Option<String>,
}

impl Display for MprisPlayerMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match (&self.artists, &self.title) {
            (None, None) => String::new(),
            (None, Some(t)) => t.clone(),
            (Some(a), None) => a.join(", "),
            (Some(a), Some(t)) => format!("{} - {}", a.join(", "), t),
        };
        write!(f, "{label}")
    }
}

impl From<HashMap<String, OwnedValue>> for MprisPlayerMetadata {
    fn from(value: HashMap<String, OwnedValue>) -> Self {
        let artists = value
            .get("xesam:artist")
            .and_then(|v| v.clone().try_into().ok());
        let title = value
            .get("xesam:title")
            .and_then(|v| v.clone().try_into().ok());

        Self { artists, title }
    }
}

/// Per-player data snapshot.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MprisPlayerData {
    pub service: String,
    pub metadata: Option<MprisPlayerMetadata>,
    pub volume: Option<f64>,
    pub state: PlaybackStatus,
    pub can_control: bool,
}

/// Complete MPRIS service state.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MprisData {
    pub players: Vec<MprisPlayerData>,
}

/// Commands for controlling a specific MPRIS player.
#[derive(Debug, Clone)]
pub struct MprisCommand {
    pub service_name: String,
    pub command: PlayerCommand,
}

/// Player control command.
#[derive(Debug, Clone, Copy)]
pub enum PlayerCommand {
    Prev,
    PlayPause,
    Next,
    Volume(f64),
}

/// Reactive MPRIS subscriber.
#[derive(Debug, Clone)]
pub struct MprisSubscriber {
    data: Mutable<MprisData>,
    conn: Connection,
}

impl MprisSubscriber {
    /// Create a new MPRIS subscriber and start monitoring player changes.
    pub async fn new() -> anyhow::Result<Self> {
        let conn = Connection::session().await?;
        let initial_data = fetch_mpris_data(&conn).await.unwrap_or_default();
        let data = Mutable::new(initial_data);

        start_listener(data.clone(), conn.clone());

        Ok(Self { data, conn })
    }

    /// Get a signal that emits when MPRIS state changes.
    pub fn subscribe(&self) -> MutableSignalCloned<MprisData> {
        self.data.signal_cloned()
    }

    /// Get the current MPRIS state snapshot.
    pub fn get(&self) -> MprisData {
        self.data.get_cloned()
    }

    /// Execute a command for a specific player.
    pub async fn dispatch(&self, command: MprisCommand) -> anyhow::Result<()> {
        let proxy = MprisPlayerProxy::builder(&self.conn)
            .destination(command.service_name.as_str())?
            .build()
            .await?;

        match command.command {
            PlayerCommand::Prev => {
                proxy.previous().await?;
            }
            PlayerCommand::PlayPause => {
                proxy.play_pause().await?;
            }
            PlayerCommand::Next => {
                proxy.next().await?;
            }
            PlayerCommand::Volume(v) => {
                let normalized = (v.clamp(0.0, 100.0)) / 100.0;
                proxy.set_volume(normalized).await?;
            }
        }

        if let Ok(new_data) = fetch_mpris_data(&self.conn).await {
            *self.data.lock_mut() = new_data;
        }

        Ok(())
    }
}

/// Fetch current MPRIS data from all available players.
async fn fetch_mpris_data(conn: &Connection) -> anyhow::Result<MprisData> {
    let dbus = DBusProxy::new(conn).await?;
    let names: Vec<String> = dbus
        .list_names()
        .await?
        .iter()
        .filter(|name| name.as_str().starts_with(MPRIS_PLAYER_SERVICE_PREFIX))
        .map(|name| name.to_string())
        .collect();

    let players = join_all(names.iter().map(|name| async move {
        let proxy = match MprisPlayerProxy::builder(conn).destination(name.as_str()) {
            Ok(builder) => match builder.build().await {
                Ok(proxy) => proxy,
                Err(_) => return None,
            },
            Err(_) => return None,
        };

        let metadata = proxy.metadata().await.ok().map(MprisPlayerMetadata::from);
        let volume = proxy.volume().await.ok().map(|v| v * 100.0);
        let state = proxy
            .playback_status()
            .await
            .map(PlaybackStatus::from)
            .unwrap_or_default();
        let can_control = proxy.can_control().await.unwrap_or(false);

        Some(MprisPlayerData {
            service: name.clone(),
            metadata,
            volume,
            state,
            can_control,
        })
    }))
    .await
    .into_iter()
    .flatten()
    .collect();

    Ok(MprisData { players })
}

/// Start the MPRIS listener in a dedicated thread.
fn start_listener(data: Mutable<MprisData>, conn: Connection) {
    thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime for MPRIS listener");

        rt.block_on(async move {
            if let Err(e) = run_listener(data, conn).await {
                error!("MPRIS listener error: {}", e);
            }
        });
    });
}

/// Run the MPRIS event listener loop.
async fn run_listener(data: Mutable<MprisData>, conn: Connection) -> anyhow::Result<()> {
    info!("MPRIS subscriber started");

    loop {
        let current = match fetch_mpris_data(&conn).await {
            Ok(value) => value,
            Err(err) => {
                error!("Failed to fetch MPRIS data: {}", err);
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                continue;
            }
        };

        if data.get_cloned() != current {
            *data.lock_mut() = current.clone();
        }

        let dbus = DBusProxy::new(&conn).await?;
        let mut streams = vec![
            dbus.receive_name_owner_changed()
                .await?
                .filter_map(|signal| async move {
                    let args = signal.args().ok()?;
                    args.name
                        .as_str()
                        .starts_with(MPRIS_PLAYER_SERVICE_PREFIX)
                        .then_some(())
                })
                .boxed(),
        ];

        let mut proxies = Vec::new();
        for player in &current.players {
            let proxy = match MprisPlayerProxy::builder(&conn).destination(player.service.as_str())
            {
                Ok(builder) => match builder.build().await {
                    Ok(proxy) => proxy,
                    Err(err) => {
                        debug!(
                            "Failed to build MPRIS proxy for {}: {}",
                            player.service, err
                        );
                        continue;
                    }
                },
                Err(err) => {
                    debug!("Invalid MPRIS destination {}: {}", player.service, err);
                    continue;
                }
            };

            let service = player.service.clone();
            streams.push(
                proxy
                    .receive_metadata_changed()
                    .await
                    .then(move |_| {
                        let service = service.clone();
                        async move {
                            trace!("MPRIS metadata changed: {}", service);
                        }
                    })
                    .boxed(),
            );

            let service = player.service.clone();
            streams.push(
                proxy
                    .receive_volume_changed()
                    .await
                    .then(move |_| {
                        let service = service.clone();
                        async move {
                            trace!("MPRIS volume changed: {}", service);
                        }
                    })
                    .boxed(),
            );

            let service = player.service.clone();
            streams.push(
                proxy
                    .receive_playback_status_changed()
                    .await
                    .then(move |_| {
                        let service = service.clone();
                        async move {
                            trace!("MPRIS playback state changed: {}", service);
                        }
                    })
                    .boxed(),
            );

            proxies.push(proxy);
        }

        let mut events = select_all(streams);
        match events.next().await {
            Some(()) => {
                // Debounce bursty property-change signals (common with some players like Spotify)
                // so we refresh once for a batch instead of once per event.
                let mut drained = 0usize;
                while let Ok(Some(())) = tokio::time::timeout(
                    tokio::time::Duration::from_millis(EVENT_DEBOUNCE_MS),
                    events.next(),
                )
                .await
                {
                    drained += 1;
                }
                trace!("MPRIS change batch received ({} extra events)", drained);
            }
            None => {
                warn!("MPRIS event stream ended unexpectedly");
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }

        drop(proxies);
    }
}

//! MPRIS media player service for playback state and controls.

mod dbus;

use std::collections::{HashMap, HashSet};
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

#[derive(Debug, Clone)]
enum MprisEvent {
    TopologyChanged,
    Metadata(String),
    Volume(String),
    Playback(String),
}

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

fn metadata_duration_us(value: &HashMap<String, OwnedValue>) -> Option<i64> {
    value
        .get("mpris:length")
        .and_then(|v| {
            v.clone()
                .try_into()
                .ok()
                .or_else(|| v.clone().try_into().ok().map(|n: u64| n as i64))
        })
        .filter(|v: &i64| *v > 0)
}

/// Per-player data snapshot.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MprisPlayerData {
    pub service: String,
    pub metadata: Option<MprisPlayerMetadata>,
    pub duration_us: Option<i64>,
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

        let metadata_raw = proxy.metadata().await.ok();
        let metadata = metadata_raw.clone().map(MprisPlayerMetadata::from);
        let duration_us = metadata_raw.as_ref().and_then(metadata_duration_us);
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
            duration_us,
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
                        .then_some(MprisEvent::TopologyChanged)
                })
                .boxed(),
        ];

        let mut proxies = HashMap::new();
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
                    .map(move |_| MprisEvent::Metadata(service.clone()))
                    .boxed(),
            );

            let service = player.service.clone();
            streams.push(
                proxy
                    .receive_volume_changed()
                    .await
                    .map(move |_| MprisEvent::Volume(service.clone()))
                    .boxed(),
            );

            let service = player.service.clone();
            streams.push(
                proxy
                    .receive_playback_status_changed()
                    .await
                    .map(move |_| MprisEvent::Playback(service.clone()))
                    .boxed(),
            );

            proxies.insert(player.service.clone(), proxy);
        }

        let mut events = select_all(streams);
        loop {
            let first = match events.next().await {
                Some(event) => event,
                None => {
                    warn!("MPRIS event stream ended unexpectedly");
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    break;
                }
            };

            let mut batch = vec![first];
            while let Ok(Some(event)) = tokio::time::timeout(
                tokio::time::Duration::from_millis(EVENT_DEBOUNCE_MS),
                events.next(),
            )
            .await
            {
                batch.push(event);
            }

            let mut topology_changed = false;
            let mut metadata_services = HashSet::new();
            let mut volume_services = HashSet::new();
            let mut playback_services = HashSet::new();

            for event in batch {
                match event {
                    MprisEvent::TopologyChanged => topology_changed = true,
                    MprisEvent::Metadata(service) => {
                        trace!("MPRIS metadata changed: {}", service);
                        metadata_services.insert(service);
                    }
                    MprisEvent::Volume(service) => {
                        trace!("MPRIS volume changed: {}", service);
                        volume_services.insert(service);
                    }
                    MprisEvent::Playback(service) => {
                        trace!("MPRIS playback changed: {}", service);
                        playback_services.insert(service);
                    }
                }
            }

            if topology_changed {
                debug!("MPRIS topology changed, rebuilding player snapshot/streams");
                break;
            }

            for service in metadata_services {
                let Some(proxy) = proxies.get(service.as_str()) else {
                    continue;
                };
                let Ok(raw) = proxy.metadata().await else {
                    continue;
                };
                let new_metadata = Some(MprisPlayerMetadata::from(raw.clone()));
                let new_duration_us = metadata_duration_us(&raw);

                let mut guard = data.lock_mut();
                if let Some(player) = guard.players.iter_mut().find(|p| p.service == service) {
                    if player.metadata != new_metadata {
                        player.metadata = new_metadata;
                    }
                    if player.duration_us != new_duration_us {
                        player.duration_us = new_duration_us;
                    }
                }
            }

            for service in volume_services {
                let Some(proxy) = proxies.get(service.as_str()) else {
                    continue;
                };
                let Ok(raw_volume) = proxy.volume().await else {
                    continue;
                };
                let new_volume = raw_volume * 100.0;

                let mut guard = data.lock_mut();
                if let Some(player) = guard.players.iter_mut().find(|p| p.service == service) {
                    let changed = player
                        .volume
                        .map(|v| (v - new_volume).abs() > 0.01)
                        .unwrap_or(true);
                    if changed {
                        player.volume = Some(new_volume);
                    }
                }
            }

            for service in playback_services {
                let Some(proxy) = proxies.get(service.as_str()) else {
                    continue;
                };
                let Ok(raw_status) = proxy.playback_status().await else {
                    continue;
                };
                let new_state = PlaybackStatus::from(raw_status);

                let mut guard = data.lock_mut();
                if let Some(player) = guard.players.iter_mut().find(|p| p.service == service)
                    && player.state != new_state
                {
                    player.state = new_state;
                }
            }
        }
    }
}

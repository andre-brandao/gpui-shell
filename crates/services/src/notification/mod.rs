//! Notification service implementing org.freedesktop.Notifications.

use std::collections::HashMap;
use std::path::PathBuf;

use chrono::Utc;
use futures_signals::signal::{Mutable, MutableSignalCloned};
use tokio::sync::broadcast;
use tracing::warn;
use zbus::{
    Connection,
    fdo::{DBusProxy, RequestNameFlags, RequestNameReply},
    interface,
    names::WellKnownName,
    object_server::SignalEmitter,
    proxy,
    zvariant::OwnedValue,
};

use crate::ServiceStatus;
use crate::applications::icons::lookup_icon;

const NAME: WellKnownName =
    WellKnownName::from_static_str_unchecked("org.freedesktop.Notifications");
const OBJECT_PATH: &str = "/org/freedesktop/Notifications";
const MAX_NOTIFICATION_HISTORY: usize = 200;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum NotificationUrgency {
    Low,
    #[default]
    Normal,
    Critical,
}

impl From<u8> for NotificationUrgency {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Low,
            2 => Self::Critical,
            _ => Self::Normal,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum NotificationTimeout {
    #[default]
    Default,
    Never,
    Millis(u64),
}

impl NotificationTimeout {
    fn from_dbus_timeout(expire_timeout: i32) -> Self {
        match expire_timeout {
            value if value < 0 => Self::Default,
            0 => Self::Never,
            value => Self::Millis(value as u64),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationCloseReason {
    Expired,
    Dismissed,
    ClosedByClient,
    Undefined,
}

impl NotificationCloseReason {
    fn dbus_reason(self) -> u32 {
        match self {
            Self::Expired => 1,
            Self::Dismissed => 2,
            Self::ClosedByClient => 3,
            Self::Undefined => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum NotificationState {
    #[default]
    Open,
    Closed {
        reason: NotificationCloseReason,
        closed_at_ms: i64,
    },
}

impl NotificationState {
    pub fn is_open(self) -> bool {
        matches!(self, Self::Open)
    }
}

#[derive(Debug, Clone, Default)]
pub struct NotificationAction {
    pub key: String,
    pub label: String,
}

#[derive(Debug, Clone)]
pub enum NotificationEvent {
    Added(Notification),
    Replaced(Notification),
    Closed {
        id: u32,
        revision: u64,
        reason: NotificationCloseReason,
    },
    Removed(u32),
    DndChanged(bool),
}

/// A single desktop notification.
#[derive(Debug, Clone, Default)]
pub struct Notification {
    pub id: u32,
    pub app_name: String,
    pub app_icon: String,
    pub revision: u64,
    pub app_icon_path: Option<PathBuf>,
    pub image_path: Option<PathBuf>,
    pub summary: String,
    pub body: String,
    pub urgency: NotificationUrgency,
    pub timeout: NotificationTimeout,
    pub timestamp_ms: i64,
    pub actions: Vec<NotificationAction>,
    pub read: bool,
    pub state: NotificationState,
}

/// Notification center state.
#[derive(Debug, Clone, Default)]
pub struct NotificationData {
    pub notifications: Vec<Notification>,
    pub dnd: bool,
    pub unread_count: usize,
}

impl NotificationData {
    fn recompute_unread(&mut self) {
        self.unread_count = self.notifications.iter().filter(|n| !n.read).count();
    }
}

/// Commands for the notification service.
#[derive(Debug, Clone)]
pub enum NotificationCommand {
    Close(u32),
    CloseLatest,
    CloseAll,
    Remove(u32),
    ClearHistory,
    SetDnd(bool),
    MarkAllRead,
    InvokeAction(u32, String),
}

/// Capacity of the notification event channel. Events are discrete (added,
/// closed, removed, ...) so they go through a broadcast channel rather than a
/// `Mutable`, which would collapse rapid events into the latest one.
const EVENT_CHANNEL_CAPACITY: usize = 64;

/// Event-driven notification service.
#[derive(Debug, Clone)]
pub struct NotificationSubscriber {
    data: Mutable<NotificationData>,
    events: broadcast::Sender<NotificationEvent>,
    status: Mutable<ServiceStatus>,
    conn: Option<Connection>,
}

impl NotificationSubscriber {
    /// Create the notification daemon and begin listening on D-Bus.
    pub async fn new() -> anyhow::Result<Self> {
        let conn = zbus::connection::Connection::session().await?;
        let data = Mutable::new(NotificationData::default());
        let (events, _) = broadcast::channel(EVENT_CHANNEL_CAPACITY);
        let status = Mutable::new(ServiceStatus::Initializing);
        let server = NotificationServer::new(data.clone(), events.clone());
        conn.object_server().at(OBJECT_PATH, server).await?;

        let dbus_proxy = DBusProxy::new(&conn).await?;
        let flags = RequestNameFlags::AllowReplacement;
        if dbus_proxy.request_name(NAME, flags.into()).await? == RequestNameReply::InQueue {
            warn!("Bus name '{NAME}' already owned, notifications will be unavailable");
            status.set(ServiceStatus::Unavailable);
            return Ok(Self {
                data,
                events,
                status,
                conn: None,
            });
        }

        status.set(ServiceStatus::Active);
        Ok(Self {
            data,
            events,
            status,
            conn: Some(conn),
        })
    }

    /// Fallback subscriber when D-Bus notification name is unavailable.
    pub fn disabled() -> Self {
        let (events, _) = broadcast::channel(EVENT_CHANNEL_CAPACITY);
        Self {
            data: Mutable::new(NotificationData::default()),
            events,
            status: Mutable::new(ServiceStatus::Unavailable),
            conn: None,
        }
    }

    pub fn subscribe(&self) -> MutableSignalCloned<NotificationData> {
        self.data.signal_cloned()
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<NotificationEvent> {
        self.events.subscribe()
    }

    pub fn get(&self) -> NotificationData {
        self.data.get_cloned()
    }

    /// Get the current service status.
    pub fn status(&self) -> ServiceStatus {
        self.status.get_cloned()
    }

    pub async fn dispatch(&self, command: NotificationCommand) -> anyhow::Result<()> {
        match command {
            NotificationCommand::Close(id) => {
                self.close_by_id(id, NotificationCloseReason::Dismissed)
                    .await?;
            }
            NotificationCommand::CloseLatest => {
                let latest = self
                    .data
                    .lock_ref()
                    .notifications
                    .iter()
                    .find(|n| n.state.is_open())
                    .map(|n| n.id);
                if let Some(id) = latest {
                    self.close_by_id(id, NotificationCloseReason::Dismissed)
                        .await?;
                }
            }
            NotificationCommand::CloseAll => {
                let ids: Vec<u32> = self
                    .data
                    .lock_ref()
                    .notifications
                    .iter()
                    .filter(|n| n.state.is_open())
                    .map(|n| n.id)
                    .collect();
                for id in ids {
                    self.close_by_id(id, NotificationCloseReason::Dismissed)
                        .await?;
                }
            }
            NotificationCommand::Remove(id) => {
                self.close_by_id(id, NotificationCloseReason::Dismissed)
                    .await?;
                if remove_notification(&self.data, id) {
                    self.publish_event(NotificationEvent::Removed(id));
                }
            }
            NotificationCommand::ClearHistory => {
                let ids: Vec<u32> = self
                    .data
                    .lock_ref()
                    .notifications
                    .iter()
                    .map(|n| n.id)
                    .collect();
                for id in &ids {
                    self.close_by_id(*id, NotificationCloseReason::Dismissed)
                        .await?;
                }
                self.data.lock_mut().notifications.clear();
                self.data.lock_mut().recompute_unread();
                for id in ids {
                    self.publish_event(NotificationEvent::Removed(id));
                }
            }
            NotificationCommand::SetDnd(enabled) => {
                self.data.lock_mut().dnd = enabled;
                self.publish_event(NotificationEvent::DndChanged(enabled));
            }
            NotificationCommand::MarkAllRead => {
                let mut data = self.data.lock_mut();
                for item in &mut data.notifications {
                    item.read = true;
                }
                data.recompute_unread();
            }
            NotificationCommand::InvokeAction(id, action_key) => {
                self.emit_action_invoked(id, &action_key).await;
                self.close_by_id(id, NotificationCloseReason::Dismissed)
                    .await?;
            }
        }

        Ok(())
    }

    async fn emit_action_invoked(&self, id: u32, action_key: &str) {
        if let Some(conn) = &self.conn
            && let Ok(iface) = conn
                .object_server()
                .interface::<_, NotificationServer>(OBJECT_PATH)
                .await
        {
            let ctx = iface.signal_emitter();
            let _ = NotificationServer::action_invoked(ctx, id, action_key).await;
        }
    }

    pub async fn close_notification(
        &self,
        id: u32,
        revision: u64,
        reason: NotificationCloseReason,
    ) -> anyhow::Result<bool> {
        let Some(closed) = close_notification(&self.data, id, Some(revision), reason) else {
            return Ok(false);
        };

        self.emit_notification_closed(id, reason).await;
        self.publish_event(NotificationEvent::Closed {
            id,
            revision: closed.revision,
            reason,
        });

        Ok(true)
    }

    fn publish_event(&self, event: NotificationEvent) {
        // Only fails when there are no receivers, which is fine.
        let _ = self.events.send(event);
    }

    async fn close_by_id(&self, id: u32, reason: NotificationCloseReason) -> anyhow::Result<bool> {
        let Some(closed) = close_notification(&self.data, id, None, reason) else {
            return Ok(false);
        };

        self.emit_notification_closed(id, reason).await;
        self.publish_event(NotificationEvent::Closed {
            id,
            revision: closed.revision,
            reason,
        });

        Ok(true)
    }

    async fn emit_notification_closed(&self, id: u32, reason: NotificationCloseReason) {
        if let Some(conn) = &self.conn
            && let Ok(iface) = conn
                .object_server()
                .interface::<_, NotificationServer>(OBJECT_PATH)
                .await
        {
            let ctx = iface.signal_emitter();
            let _ = NotificationServer::notification_closed(ctx, id, reason.dbus_reason()).await;
        }
    }
}

impl Default for NotificationSubscriber {
    fn default() -> Self {
        Self::disabled()
    }
}

#[derive(Debug)]
struct NotificationServer {
    data: Mutable<NotificationData>,
    events: broadcast::Sender<NotificationEvent>,
    next_id: u32,
    next_revision: u64,
}

impl NotificationServer {
    fn new(data: Mutable<NotificationData>, events: broadcast::Sender<NotificationEvent>) -> Self {
        Self {
            data,
            events,
            next_id: 1,
            next_revision: 1,
        }
    }

    fn publish_event(&self, event: NotificationEvent) {
        // Only fails when there are no receivers, which is fine.
        let _ = self.events.send(event);
    }
}

#[interface(
    name = "org.freedesktop.Notifications",
    proxy(
        gen_blocking = false,
        default_service = "org.freedesktop.Notifications",
        default_path = "/org/freedesktop/Notifications",
    )
)]
impl NotificationServer {
    #[zbus(name = "GetCapabilities")]
    async fn get_capabilities(&self) -> Vec<String> {
        vec![
            "actions".to_string(),
            "body".to_string(),
            "body-markup".to_string(),
            "persistence".to_string(),
        ]
    }

    #[zbus(name = "GetServerInformation")]
    async fn get_server_information(&self) -> (String, String, String, String) {
        (
            "GPUi Shell".to_string(),
            "gpuishell".to_string(),
            "0.1.0".to_string(),
            "1.2".to_string(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    #[zbus(name = "Notify")]
    async fn notify(
        &mut self,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: Vec<String>,
        hints: HashMap<String, OwnedValue>,
        expire_timeout: i32,
    ) -> u32 {
        let replaces_existing = replaces_id != 0
            && self
                .data
                .lock_ref()
                .notifications
                .iter()
                .any(|n| n.id == replaces_id);
        let id = if replaces_existing {
            replaces_id
        } else {
            let id = self.next_id;
            self.next_id = self.next_id.saturating_add(1);
            id
        };
        let revision = self.next_revision;
        self.next_revision = self.next_revision.saturating_add(1);

        let urgency = hints
            .get("urgency")
            .and_then(|v| u8::try_from(v.clone()).ok())
            .map(NotificationUrgency::from)
            .unwrap_or_default();
        let image_path =
            hint_string(&hints, &["image-path", "image_path"]).map(|p| normalize_path(&p));
        let app_icon_path = if is_image_source(app_icon) {
            Some(normalize_path(app_icon))
        } else {
            hint_string(&hints, &["app_icon", "icon-path", "icon_path"])
                .filter(|value| is_image_source(value))
                .map(|p| normalize_path(&p))
        };
        // Fallback: resolve named icon via XDG icon theme lookup
        let app_icon_path = app_icon_path.or_else(|| {
            if !app_icon.is_empty() {
                lookup_icon(app_icon)
            } else {
                None
            }
        });
        // Fallback: try desktop-entry hint for icon lookup
        let app_icon_path = app_icon_path.or_else(|| {
            hint_string(&hints, &["desktop-entry"]).and_then(|entry| lookup_icon(&entry))
        });
        let timeout = NotificationTimeout::from_dbus_timeout(expire_timeout);
        let parsed_actions = actions
            .chunks(2)
            .filter_map(|chunk| match chunk {
                [key, label] => Some(NotificationAction {
                    key: key.clone(),
                    label: label.clone(),
                }),
                _ => None,
            })
            .collect();

        let notification = Notification {
            id,
            app_name: app_name.to_string(),
            app_icon: app_icon.to_string(),
            revision,
            app_icon_path,
            image_path,
            summary: summary.to_string(),
            body: body.to_string(),
            urgency,
            timeout,
            timestamp_ms: Utc::now().timestamp_millis(),
            actions: parsed_actions,
            read: false,
            state: NotificationState::Open,
        };

        {
            let mut data = self.data.lock_mut();
            data.notifications.retain(|n| n.id != id);
            data.notifications.insert(0, notification.clone());
            if data.notifications.len() > MAX_NOTIFICATION_HISTORY {
                data.notifications.truncate(MAX_NOTIFICATION_HISTORY);
            }
            data.recompute_unread();
        }

        self.publish_event(if replaces_existing {
            NotificationEvent::Replaced(notification)
        } else {
            NotificationEvent::Added(notification)
        });

        id
    }

    #[zbus(name = "CloseNotification")]
    async fn close_notification(
        &mut self,
        id: u32,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) {
        if let Some(closed) = close_notification(
            &self.data,
            id,
            None,
            NotificationCloseReason::ClosedByClient,
        ) {
            let reason = NotificationCloseReason::ClosedByClient;
            let _ =
                NotificationServer::notification_closed(&emitter, id, reason.dbus_reason()).await;
            self.publish_event(NotificationEvent::Closed {
                id,
                revision: closed.revision,
                reason,
            });
        }
    }

    #[zbus(signal, name = "NotificationClosed")]
    async fn notification_closed(
        emitter: &SignalEmitter<'_>,
        id: u32,
        reason: u32,
    ) -> zbus::Result<()>;

    #[zbus(signal, name = "ActionInvoked")]
    async fn action_invoked(
        emitter: &SignalEmitter<'_>,
        id: u32,
        action_key: &str,
    ) -> zbus::Result<()>;
}

fn remove_notification(data: &Mutable<NotificationData>, id: u32) -> bool {
    let mut state = data.lock_mut();
    let len_before = state.notifications.len();
    state.notifications.retain(|n| n.id != id);
    state.recompute_unread();
    len_before != state.notifications.len()
}

fn close_notification(
    data: &Mutable<NotificationData>,
    id: u32,
    revision: Option<u64>,
    reason: NotificationCloseReason,
) -> Option<Notification> {
    let mut state = data.lock_mut();
    let notification = state
        .notifications
        .iter_mut()
        .find(|notification| notification.id == id)?;

    if revision.is_some_and(|revision| notification.revision != revision)
        || !notification.state.is_open()
    {
        return None;
    }

    notification.state = NotificationState::Closed {
        reason,
        closed_at_ms: Utc::now().timestamp_millis(),
    };
    Some(notification.clone())
}

fn hint_string(hints: &HashMap<String, OwnedValue>, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        hints.get(*key).and_then(|v| {
            String::try_from(v.clone())
                .ok()
                .or_else(|| {
                    let raw = format!("{v:?}");
                    raw.strip_prefix('"')
                        .and_then(|s| s.strip_suffix('"'))
                        .map(ToString::to_string)
                })
                .filter(|s| !s.trim().is_empty())
        })
    })
}

fn is_image_source(value: &str) -> bool {
    value.starts_with('/')
        || value.starts_with("file://")
        || value.starts_with("http://")
        || value.starts_with("https://")
}

fn normalize_path(value: &str) -> PathBuf {
    let path = value.strip_prefix("file://").unwrap_or(value);
    let bytes = path.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(h), Some(l)) = (hex_nibble(bytes[i + 1]), hex_nibble(bytes[i + 2]))
        {
            out.push((h << 4) | l);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    PathBuf::from(String::from_utf8_lossy(&out).into_owned())
}

fn hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[proxy(
    interface = "org.freedesktop.Notifications",
    default_service = "org.freedesktop.Notifications",
    default_path = "/org/freedesktop/Notifications",
    gen_blocking = false
)]
trait Notifications {
    #[zbus(name = "CloseNotification")]
    fn close_notification(&self, id: u32) -> zbus::Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_path_unchanged() {
        assert_eq!(
            normalize_path("/home/user/pic.png"),
            PathBuf::from("/home/user/pic.png")
        );
    }

    #[test]
    fn strips_file_scheme() {
        assert_eq!(
            normalize_path("file:///home/user/pic.png"),
            PathBuf::from("/home/user/pic.png")
        );
    }

    #[test]
    fn decodes_percent_space() {
        assert_eq!(
            normalize_path("file:///home/user/My%20Pictures/test.png"),
            PathBuf::from("/home/user/My Pictures/test.png"),
        );
    }

    #[test]
    fn decodes_utf8_sequence() {
        assert_eq!(
            normalize_path("/tmp/%C3%A9.png"),
            PathBuf::from("/tmp/é.png")
        );
    }

    #[test]
    fn decodes_lowercase_hex() {
        assert_eq!(normalize_path("/a/b%2fc"), PathBuf::from("/a/b/c"));
    }

    #[test]
    fn keeps_invalid_hex_literal() {
        assert_eq!(normalize_path("/a/%ZZ/b"), PathBuf::from("/a/%ZZ/b"));
    }

    #[test]
    fn keeps_trailing_short_percent() {
        assert_eq!(normalize_path("/a/%2"), PathBuf::from("/a/%2"));
        assert_eq!(normalize_path("/a/%"), PathBuf::from("/a/%"));
    }

    #[test]
    fn mixed_encoding() {
        assert_eq!(
            normalize_path("file:///tmp/My%20%C3%A9.png"),
            PathBuf::from("/tmp/My é.png"),
        );
    }

    /// Rapid events must all reach subscribers — a `Mutable` would collapse
    /// them into the latest value (the bug this guards against).
    #[tokio::test]
    async fn rapid_events_are_not_dropped() {
        let subscriber = NotificationSubscriber::disabled();
        let mut events = subscriber.subscribe_events();

        const N: usize = 10;
        for i in 0..N {
            subscriber
                .dispatch(NotificationCommand::SetDnd(i % 2 == 0))
                .await
                .unwrap();
        }

        for i in 0..N {
            match events.try_recv() {
                Ok(NotificationEvent::DndChanged(enabled)) => assert_eq!(enabled, i % 2 == 0),
                other => panic!("expected DndChanged event #{i}, got {other:?}"),
            }
        }
    }
}

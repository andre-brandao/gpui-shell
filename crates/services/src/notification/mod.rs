//! Notification service implementing org.freedesktop.Notifications.

use std::collections::HashMap;
use std::path::PathBuf;

use chrono::Utc;
use futures_signals::signal::{Mutable, MutableSignalCloned};
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
const DEFAULT_TIMEOUT_MS: i32 = 5000;
const MAX_NOTIFICATION_HISTORY: usize = 200;

/// A single desktop notification.
#[derive(Debug, Clone, Default)]
pub struct Notification {
    pub id: u32,
    pub app_name: String,
    pub app_icon: String,
    pub app_icon_path: Option<PathBuf>,
    pub image_path: Option<PathBuf>,
    pub summary: String,
    pub body: String,
    pub urgency: u8,
    pub timeout_ms: i32,
    pub timestamp_ms: i64,
    pub actions: Vec<(String, String)>,
    pub read: bool,
    pub closed: bool,
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
    Dismiss(u32),
    DismissLatest,
    DismissAll,
    SetDnd(bool),
    MarkAllRead,
    InvokeAction(u32, String),
}

/// Event-driven notification service.
#[derive(Debug, Clone)]
pub struct NotificationSubscriber {
    data: Mutable<NotificationData>,
    status: Mutable<ServiceStatus>,
    conn: Option<Connection>,
}

impl NotificationSubscriber {
    /// Create the notification daemon and begin listening on D-Bus.
    pub async fn new() -> anyhow::Result<Self> {
        let conn = zbus::connection::Connection::session().await?;
        let data = Mutable::new(NotificationData::default());
        let status = Mutable::new(ServiceStatus::Initializing);
        let server = NotificationServer::new(data.clone());
        conn.object_server().at(OBJECT_PATH, server).await?;

        let dbus_proxy = DBusProxy::new(&conn).await?;
        let flags = RequestNameFlags::AllowReplacement;
        if dbus_proxy.request_name(NAME, flags.into()).await? == RequestNameReply::InQueue {
            warn!("Bus name '{NAME}' already owned, notifications will be unavailable");
            status.set(ServiceStatus::Unavailable);
            return Ok(Self {
                data,
                status,
                conn: None,
            });
        }

        status.set(ServiceStatus::Active);
        Ok(Self {
            data,
            status,
            conn: Some(conn),
        })
    }

    /// Fallback subscriber when D-Bus notification name is unavailable.
    pub fn disabled() -> Self {
        Self {
            data: Mutable::new(NotificationData::default()),
            status: Mutable::new(ServiceStatus::Unavailable),
            conn: None,
        }
    }

    pub fn subscribe(&self) -> MutableSignalCloned<NotificationData> {
        self.data.signal_cloned()
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
            NotificationCommand::Dismiss(id) => {
                self.dismiss_by_id(id).await?;
            }
            NotificationCommand::DismissLatest => {
                if let Some(id) = self.data.lock_ref().notifications.first().map(|n| n.id) {
                    self.dismiss_by_id(id).await?;
                }
            }
            NotificationCommand::DismissAll => {
                let ids: Vec<u32> = self
                    .data
                    .lock_ref()
                    .notifications
                    .iter()
                    .map(|n| n.id)
                    .collect();
                for id in ids {
                    self.dismiss_by_id(id).await?;
                }
            }
            NotificationCommand::SetDnd(enabled) => {
                self.data.lock_mut().dnd = enabled;
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
                self.dismiss_by_id(id).await?;
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

    pub async fn expire_notification(&self, id: u32, timestamp_ms: i64) -> anyhow::Result<bool> {
        if !close_notification(&self.data, id, Some(timestamp_ms)) {
            return Ok(false);
        }

        if let Some(conn) = &self.conn
            && let Ok(iface) = conn
                .object_server()
                .interface::<_, NotificationServer>(OBJECT_PATH)
                .await
        {
            let ctx = iface.signal_emitter();
            let _ = NotificationServer::notification_closed(ctx, id, 1).await;
        }

        Ok(true)
    }

    async fn dismiss_by_id(&self, id: u32) -> anyhow::Result<()> {
        if let Some(conn) = &self.conn {
            let proxy = NotificationsProxy::new(conn).await?;
            let _ = proxy.close_notification(id).await;
        }
        remove_notification(&self.data, id);
        Ok(())
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
    next_id: u32,
}

impl NotificationServer {
    fn new(data: Mutable<NotificationData>) -> Self {
        Self { data, next_id: 1 }
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
        let id = {
            let existing = self
                .data
                .lock_ref()
                .notifications
                .iter()
                .any(|n| n.id == replaces_id);
            if replaces_id != 0 && existing {
                replaces_id
            } else {
                let id = self.next_id;
                self.next_id = self.next_id.saturating_add(1);
                id
            }
        };

        let urgency = hints
            .get("urgency")
            .and_then(|v| u8::try_from(v.clone()).ok())
            .unwrap_or(1);
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
        let timeout_ms = if expire_timeout < 0 {
            DEFAULT_TIMEOUT_MS
        } else {
            expire_timeout
        };
        let parsed_actions = actions
            .chunks(2)
            .filter_map(|chunk| match chunk {
                [key, label] => Some((key.clone(), label.clone())),
                _ => None,
            })
            .collect();

        let notification = Notification {
            id,
            app_name: app_name.to_string(),
            app_icon: app_icon.to_string(),
            app_icon_path,
            image_path,
            summary: summary.to_string(),
            body: body.to_string(),
            urgency,
            timeout_ms,
            timestamp_ms: Utc::now().timestamp_millis(),
            actions: parsed_actions,
            read: false,
            closed: false,
        };

        {
            let mut data = self.data.lock_mut();
            data.notifications.retain(|n| n.id != id);
            data.notifications.insert(0, notification);
            if data.notifications.len() > MAX_NOTIFICATION_HISTORY {
                data.notifications.truncate(MAX_NOTIFICATION_HISTORY);
            }
            data.recompute_unread();
        }

        id
    }

    #[zbus(name = "CloseNotification")]
    async fn close_notification(
        &mut self,
        id: u32,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) {
        if close_notification(&self.data, id, None) {
            let _ = NotificationServer::notification_closed(&emitter, id, 2).await;
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
    timestamp_ms: Option<i64>,
) -> bool {
    let mut state = data.lock_mut();
    let Some(notification) = state
        .notifications
        .iter_mut()
        .find(|notification| notification.id == id)
    else {
        return false;
    };

    if timestamp_ms.is_some_and(|timestamp_ms| notification.timestamp_ms != timestamp_ms)
        || notification.closed
    {
        return false;
    }

    notification.closed = true;
    true
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
}

//! Notification service implementing org.freedesktop.Notifications.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

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
use crate::lifecycle::{Lifecycle, ManagedService};

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
}

/// Notification center state.
#[derive(Debug, Clone, Default)]
pub struct NotificationData {
    pub notifications: Vec<Notification>,
    pub popup_ids: Vec<u32>,
    pub dnd: bool,
    pub unread_count: usize,
}

impl NotificationData {
    fn recompute_unread(&mut self) {
        self.unread_count = self.notifications.iter().filter(|n| !n.read).count();
    }

    fn latest_popup(&self) -> Option<Notification> {
        let id = self.popup_ids.first().copied()?;
        self.notifications.iter().find(|n| n.id == id).cloned()
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
#[derive(Debug, Clone, Default)]
pub struct NotificationSubscriber {
    data: Mutable<NotificationData>,
    lifecycle: Lifecycle,
}

impl NotificationSubscriber {
    /// Create the notification subscriber. The D-Bus daemon is claimed by
    /// [`ManagedService::start`].
    pub fn new() -> Self {
        Self::default()
    }

    pub fn subscribe(&self) -> MutableSignalCloned<NotificationData> {
        self.data.signal_cloned()
    }

    pub fn get(&self) -> NotificationData {
        self.data.get_cloned()
    }

    pub fn latest_popup(&self) -> Option<Notification> {
        self.data.lock_ref().latest_popup()
    }

    pub fn popup_notifications(&self, limit: usize) -> Vec<Notification> {
        let data = self.data.lock_ref();
        data.popup_ids
            .iter()
            .filter_map(|id| data.notifications.iter().find(|n| n.id == *id).cloned())
            .take(limit)
            .collect()
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
                let mut data = self.data.lock_mut();
                data.dnd = enabled;
                if enabled {
                    data.popup_ids.clear();
                }
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
        let Ok(conn) = crate::bus::session().await else {
            return;
        };
        if let Ok(iface) = conn
            .object_server()
            .interface::<_, NotificationServer>(OBJECT_PATH)
            .await
        {
            let ctx = iface.signal_emitter();
            let _ = NotificationServer::action_invoked(ctx, id, action_key).await;
        }
    }

    async fn dismiss_by_id(&self, id: u32) -> anyhow::Result<()> {
        if self.lifecycle.is_up() {
            let conn = crate::bus::session().await?;
            let proxy = NotificationsProxy::new(&conn).await?;
            let _ = proxy.close_notification(id).await;
        }
        remove_notification(&self.data, id);
        Ok(())
    }
}

impl ManagedService for NotificationSubscriber {
    fn name(&self) -> &'static str {
        "Notifications"
    }

    fn lifecycle(&self) -> &Lifecycle {
        &self.lifecycle
    }

    /// Notification bodies and their action labels are kept in history, so
    /// they dominate what this service retains.
    fn memory_bytes(&self) -> usize {
        let data = self.data.lock_ref();
        size_of::<NotificationData>()
            + data.popup_ids.len() * size_of::<u32>()
            + data
                .notifications
                .iter()
                .map(|notification| {
                    size_of::<Notification>()
                        + notification.app_name.len()
                        + notification.app_icon.len()
                        + notification.summary.len()
                        + notification.body.len()
                        + notification
                            .app_icon_path
                            .as_ref()
                            .map_or(0, |path| path.as_os_str().len())
                        + notification
                            .image_path
                            .as_ref()
                            .map_or(0, |path| path.as_os_str().len())
                        + notification
                            .actions
                            .iter()
                            .map(|(key, label)| key.len() + label.len())
                            .sum::<usize>()
                })
                .sum::<usize>()
    }

    fn start(&self) {
        let token = self.lifecycle.begin();
        let data = self.data.clone();

        tokio::spawn(async move {
            let conn = match crate::bus::session().await {
                Ok(conn) => conn,
                Err(e) => {
                    warn!("Failed to connect to the session bus: {e}");
                    token.error(e.to_string());
                    return;
                }
            };

            // A previous run may still own the object path.
            let _ = conn
                .object_server()
                .remove::<NotificationServer, _>(OBJECT_PATH)
                .await;

            let server = NotificationServer::new(data, conn.clone());
            if let Err(e) = conn.object_server().at(OBJECT_PATH, server).await {
                warn!("Failed to serve the notification interface: {e}");
                token.error(e.to_string());
                return;
            }

            let flags = RequestNameFlags::AllowReplacement;
            match DBusProxy::new(&conn).await {
                Ok(proxy) => match proxy.request_name(NAME, flags.into()).await {
                    Ok(RequestNameReply::InQueue) => {
                        warn!("Bus name '{NAME}' already owned, notifications unavailable");
                        token.set(ServiceStatus::Unavailable);
                    }
                    Ok(_) => token.active(),
                    Err(e) => {
                        warn!("Failed to request bus name '{NAME}': {e}");
                        token.error(e.to_string());
                    }
                },
                Err(e) => {
                    warn!("Failed to reach the session bus: {e}");
                    token.error(e.to_string());
                }
            }
        });
    }

    /// Releases the bus name as well: a stopped notification daemon must let
    /// another one take over.
    fn stop(&self) {
        self.lifecycle.stop();

        tokio::spawn(async move {
            let Ok(conn) = crate::bus::session().await else {
                return;
            };
            let _ = conn
                .object_server()
                .remove::<NotificationServer, _>(OBJECT_PATH)
                .await;
            if let Ok(proxy) = DBusProxy::new(&conn).await {
                let _ = proxy.release_name(NAME).await;
            }
        });
    }
}

#[derive(Debug)]
struct NotificationServer {
    data: Mutable<NotificationData>,
    conn: Connection,
    next_id: u32,
    next_timer_generation: u64,
    timer_generations: Arc<Mutex<HashMap<u32, u64>>>,
}

impl NotificationServer {
    fn new(data: Mutable<NotificationData>, conn: Connection) -> Self {
        Self {
            data,
            conn,
            next_id: 1,
            next_timer_generation: 1,
            timer_generations: Arc::new(Mutex::new(HashMap::new())),
        }
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
        // Some senders (e.g. Chromium) write the icon/image to a temp file and
        // unlink it shortly after sending the notification, so only keep paths
        // that still exist by the time we parse the hints - otherwise gpui's
        // asset cache logs a load error for a file that's already gone.
        let image_path = hint_string(&hints, &["image-path", "image_path"])
            .map(|p| normalize_path(&p))
            .filter(|p| p.exists());
        let app_icon_path = if is_image_source(app_icon) {
            Some(normalize_path(app_icon)).filter(|p| p.exists())
        } else {
            hint_string(&hints, &["app_icon", "icon-path", "icon_path"])
                .filter(|value| is_image_source(value))
                .map(|p| normalize_path(&p))
                .filter(|p| p.exists())
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
        };

        {
            let mut data = self.data.lock_mut();
            data.notifications.retain(|n| n.id != id);
            data.popup_ids.retain(|n| *n != id);
            data.notifications.insert(0, notification);
            if data.notifications.len() > MAX_NOTIFICATION_HISTORY {
                data.notifications.truncate(MAX_NOTIFICATION_HISTORY);
            }
            if !data.dnd {
                data.popup_ids.insert(0, id);
            }
            data.recompute_unread();
        }

        if timeout_ms > 0 {
            self.next_timer_generation = self.next_timer_generation.saturating_add(1);
            let generation = self.next_timer_generation;
            if let Ok(mut timers) = self.timer_generations.lock() {
                timers.insert(id, generation);
            }

            let conn = self.conn.clone();
            let data = self.data.clone();
            let timer_generations = self.timer_generations.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(timeout_ms as u64)).await;

                let should_close = timer_generations
                    .lock()
                    .ok()
                    .and_then(|map| map.get(&id).copied())
                    .map(|current_generation| current_generation == generation)
                    .unwrap_or(false);
                if !should_close {
                    return;
                }

                if let Ok(mut timers) = timer_generations.lock() {
                    timers.remove(&id);
                }

                if !deactivate_notification(&data, id) {
                    return;
                }

                // Emit NotificationClosed with reason 1 (expired)
                if let Ok(iface) = conn
                    .object_server()
                    .interface::<_, NotificationServer>(OBJECT_PATH)
                    .await
                {
                    let ctx = iface.signal_emitter();
                    let _ = NotificationServer::notification_closed(ctx, id, 1).await;
                }
            });
        }

        id
    }

    #[zbus(name = "CloseNotification")]
    async fn close_notification(
        &mut self,
        id: u32,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) {
        if let Ok(mut timers) = self.timer_generations.lock() {
            timers.remove(&id);
        }

        if deactivate_notification(&self.data, id) {
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
    state.popup_ids.retain(|x| *x != id);
    state.recompute_unread();
    len_before != state.notifications.len()
}

fn deactivate_notification(data: &Mutable<NotificationData>, id: u32) -> bool {
    let mut state = data.lock_mut();
    let had_popup = state.popup_ids.contains(&id);
    state.popup_ids.retain(|x| *x != id);
    had_popup || state.notifications.iter().any(|n| n.id == id)
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

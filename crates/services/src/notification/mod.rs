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

use crate::applications::icons::lookup_icon;

const NAME: WellKnownName =
    WellKnownName::from_static_str_unchecked("org.freedesktop.Notifications");
const OBJECT_PATH: &str = "/org/freedesktop/Notifications";
const DEFAULT_TIMEOUT_MS: i32 = 5000;

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
#[derive(Debug, Clone)]
pub struct NotificationSubscriber {
    data: Mutable<NotificationData>,
    conn: Option<Connection>,
}

impl NotificationSubscriber {
    /// Create the notification daemon and begin listening on D-Bus.
    pub async fn new() -> anyhow::Result<Self> {
        let conn = zbus::connection::Connection::session().await?;
        let data = Mutable::new(NotificationData::default());
        let server = NotificationServer::new(data.clone(), conn.clone());
        conn.object_server().at(OBJECT_PATH, server).await?;

        let dbus_proxy = DBusProxy::new(&conn).await?;
        let flags = RequestNameFlags::AllowReplacement;
        if dbus_proxy.request_name(NAME, flags.into()).await? == RequestNameReply::InQueue {
            warn!("Bus name '{NAME}' already owned, notifications will be unavailable");
            return Ok(Self { data, conn: None });
        }

        Ok(Self {
            data,
            conn: Some(conn),
        })
    }

    /// Fallback subscriber when D-Bus notification name is unavailable.
    pub fn disabled() -> Self {
        Self {
            data: Mutable::new(NotificationData::default()),
            conn: None,
        }
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
        };

        {
            let mut data = self.data.lock_mut();
            data.notifications.retain(|n| n.id != id);
            data.popup_ids.retain(|n| *n != id);
            data.notifications.insert(0, notification);
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
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(timeout_ms as u64));

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
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("failed to build tokio runtime for notification timeout");
                rt.block_on(async move {
                    if let Ok(iface) = conn
                        .object_server()
                        .interface::<_, NotificationServer>(OBJECT_PATH)
                        .await
                    {
                        let ctx = iface.signal_emitter();
                        let _ = NotificationServer::notification_closed(ctx, id, 1).await;
                    }
                });
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
    let had_popup = state.popup_ids.iter().any(|x| *x == id);
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

/// Normalize a file path from D-Bus hints, handling file:// URIs and URL encoding.
fn normalize_path(value: &str) -> PathBuf {
    let path = value.strip_prefix("file://").unwrap_or(value);

    // Simple URL decode for common cases (%20 -> space, etc.)
    let mut result = String::with_capacity(path.len());
    let mut chars = path.chars();

    while let Some(ch) = chars.next() {
        if ch == '%' {
            // Try to decode percent-encoded character
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() == 2 {
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                    continue;
                }
            }
            // If decoding fails, just keep the original
            result.push('%');
            result.push_str(&hex);
        } else {
            result.push(ch);
        }
    }

    PathBuf::from(result)
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

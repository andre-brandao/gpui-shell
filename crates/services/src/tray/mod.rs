//! System tray service using StatusNotifierItem protocol.
//!
//! This module provides a reactive subscriber for monitoring system tray items
//! via the StatusNotifierItem/StatusNotifierWatcher D-Bus protocol.

mod dbus;

pub use dbus::{MenuLayout, MenuLayoutProps};

use std::sync::Arc;
use std::thread;

use dbus::{
    DBusMenuProxy, StatusNotifierItemProxy, StatusNotifierWatcher, StatusNotifierWatcherProxy,
};
use futures_signals::signal::{Mutable, MutableSignalCloned};
use futures_util::StreamExt;
use futures_util::stream::select_all;
use tracing::{debug, error, info};

/// Icon data for a tray item.
#[derive(Debug, Clone)]
pub enum TrayIcon {
    /// Icon name for lookup via freedesktop icon theme.
    Name(String),
    /// RGBA pixel data.
    Pixmap {
        width: u32,
        height: u32,
        data: Arc<Vec<u8>>,
    },
}

/// A system tray item.
#[derive(Debug, Clone)]
pub struct TrayItem {
    /// Unique identifier (D-Bus service name).
    pub name: String,
    /// Display title.
    pub title: Option<String>,
    /// Application ID.
    pub id: Option<String>,
    /// Icon for the tray item.
    pub icon: Option<TrayIcon>,
    /// Menu layout.
    pub menu: Option<MenuLayout>,
    /// Internal: D-Bus destination for commands.
    dest: String,
    /// Internal: Menu path.
    menu_path: String,
}

impl TrayItem {
    /// Get the icon name if available.
    pub fn icon_name(&self) -> Option<&str> {
        match &self.icon {
            Some(TrayIcon::Name(name)) => Some(name),
            _ => None,
        }
    }

    /// Get pixmap data if available.
    pub fn icon_pixmap(&self) -> Option<(u32, u32, &[u8])> {
        match &self.icon {
            Some(TrayIcon::Pixmap {
                width,
                height,
                data,
            }) => Some((*width, *height, data.as_slice())),
            _ => None,
        }
    }
}

/// System tray data.
#[derive(Debug, Clone, Default)]
pub struct TrayData {
    /// List of tray items.
    pub items: Vec<TrayItem>,
}

impl TrayData {
    /// Find a tray item by name.
    pub fn find(&self, name: &str) -> Option<&TrayItem> {
        self.items.iter().find(|item| item.name == name)
    }

    /// Check if there are any tray items.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Get the number of tray items.
    pub fn len(&self) -> usize {
        self.items.len()
    }
}

/// Commands for the tray service.
#[derive(Debug, Clone)]
pub enum TrayCommand {
    /// Activate a menu item.
    MenuItemClicked { item_name: String, menu_id: i32 },
}

/// Event-driven system tray subscriber.
#[derive(Debug, Clone)]
pub struct TraySubscriber {
    data: Mutable<TrayData>,
    conn: zbus::Connection,
}

impl TraySubscriber {
    /// Create a new tray subscriber and start monitoring.
    pub async fn new() -> anyhow::Result<Self> {
        // Start the StatusNotifierWatcher server
        let conn = StatusNotifierWatcher::start_server().await?;

        let initial_data = fetch_tray_data(&conn).await.unwrap_or_default();
        let data = Mutable::new(initial_data);

        info!(
            "Tray service initialized with {} items",
            data.lock_ref().items.len()
        );

        // Start the event listener
        start_listener(data.clone(), conn.clone());

        Ok(Self { data, conn })
    }

    /// Get a signal that emits when tray state changes.
    pub fn subscribe(&self) -> MutableSignalCloned<TrayData> {
        self.data.signal_cloned()
    }

    /// Get the current tray data snapshot.
    pub fn get(&self) -> TrayData {
        self.data.get_cloned()
    }

    /// Execute a tray command.
    pub async fn dispatch(&self, command: TrayCommand) -> anyhow::Result<()> {
        match command {
            TrayCommand::MenuItemClicked { item_name, menu_id } => {
                let data = self.data.lock_ref();
                if let Some(item) = data.items.iter().find(|i| i.name == item_name) {
                    let menu_proxy = DBusMenuProxy::builder(&self.conn)
                        .destination(item.dest.clone())?
                        .path(item.menu_path.clone())?
                        .build()
                        .await?;

                    debug!("Clicking menu item {} in {}", menu_id, item_name);

                    let value = zbus::zvariant::Value::I32(0).try_to_owned()?;
                    let timestamp = chrono::Local::now().timestamp_subsec_millis();
                    menu_proxy
                        .event(menu_id, "clicked", &value, timestamp)
                        .await?;

                    // Refresh menu layout after click
                    drop(data);
                    if let Ok((_, new_layout)) = menu_proxy.get_layout(0, -1, &[]).await {
                        let mut data = self.data.lock_mut();
                        if let Some(item) = data.items.iter_mut().find(|i| i.name == item_name) {
                            item.menu = Some(new_layout);
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

/// Fetch current tray data.
async fn fetch_tray_data(conn: &zbus::Connection) -> anyhow::Result<TrayData> {
    let proxy = StatusNotifierWatcherProxy::new(conn).await?;
    let items = proxy.registered_status_notifier_items().await?;

    let mut tray_items = Vec::with_capacity(items.len());
    for name in items {
        match create_tray_item(conn, &name).await {
            Ok(item) => tray_items.push(item),
            Err(e) => debug!("Failed to create tray item {}: {}", name, e),
        }
    }

    Ok(TrayData { items: tray_items })
}

/// Create a TrayItem from a StatusNotifierItem.
async fn create_tray_item(conn: &zbus::Connection, name: &str) -> anyhow::Result<TrayItem> {
    let (dest, path) = if let Some(idx) = name.find('/') {
        (&name[..idx], &name[idx..])
    } else {
        (name, "/StatusNotifierItem")
    };

    let item_proxy = StatusNotifierItemProxy::builder(conn)
        .destination(dest.to_owned())?
        .path(path.to_owned())?
        .build()
        .await?;

    // Get icon - try pixmap first, then name
    let icon = match item_proxy.icon_pixmap().await {
        Ok(icons) => {
            icons
                .into_iter()
                .max_by_key(|i| (i.width, i.height))
                .map(|mut i| {
                    // Convert ARGB to RGBA
                    for pixel in i.bytes.chunks_exact_mut(4) {
                        pixel.rotate_left(1);
                    }
                    TrayIcon::Pixmap {
                        width: i.width as u32,
                        height: i.height as u32,
                        data: Arc::new(i.bytes),
                    }
                })
        }
        Err(_) => item_proxy
            .icon_name()
            .await
            .ok()
            .filter(|n| !n.is_empty())
            .map(TrayIcon::Name),
    };

    let title = item_proxy.title().await.ok();
    let id = item_proxy.id().await.ok();

    // Get menu
    let menu_path = item_proxy.menu().await?;
    let menu_path_str = menu_path.to_string();

    let menu = if !menu_path_str.is_empty() && menu_path_str != "/" {
        let menu_proxy = DBusMenuProxy::builder(conn)
            .destination(dest.to_owned())?
            .path(menu_path.clone())?
            .build()
            .await?;

        menu_proxy.get_layout(0, -1, &[]).await.ok().map(|(_, l)| l)
    } else {
        None
    };

    Ok(TrayItem {
        name: name.to_string(),
        title,
        id,
        icon,
        menu,
        dest: dest.to_string(),
        menu_path: menu_path_str,
    })
}

/// Start the D-Bus listener in a dedicated thread.
fn start_listener(data: Mutable<TrayData>, conn: zbus::Connection) {
    thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime for Tray listener");

        rt.block_on(async move {
            loop {
                if let Err(e) = run_listener(&data, &conn).await {
                    error!("Tray listener error: {}", e);
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
            }
        });
    });
}

/// Run the tray event listener.
async fn run_listener(data: &Mutable<TrayData>, conn: &zbus::Connection) -> anyhow::Result<()> {
    let watcher = StatusNotifierWatcherProxy::new(conn).await?;

    // Stream for item registered
    let data_reg = data.clone();
    let conn_reg = conn.clone();
    let registered = watcher
        .receive_status_notifier_item_registered()
        .await?
        .filter_map(move |e| {
            let data = data_reg.clone();
            let conn = conn_reg.clone();
            async move {
                if let Ok(args) = e.args() {
                    let name = args.service.to_string();
                    debug!("Tray item registered: {}", name);

                    if let Ok(item) = create_tray_item(&conn, &name).await {
                        let mut guard = data.lock_mut();
                        // Update or add
                        if let Some(existing) = guard.items.iter_mut().find(|i| i.name == name) {
                            *existing = item;
                        } else {
                            guard.items.push(item);
                        }
                    }
                }
                Some(())
            }
        })
        .boxed();

    // Stream for item unregistered
    let data_unreg = data.clone();
    let unregistered = watcher
        .receive_status_notifier_item_unregistered()
        .await?
        .filter_map(move |e| {
            let data = data_unreg.clone();
            async move {
                if let Ok(args) = e.args() {
                    let name = args.service.to_string();
                    debug!("Tray item unregistered: {}", name);
                    data.lock_mut().items.retain(|item| item.name != name);
                }
                Some(())
            }
        })
        .boxed();

    // Set up icon and menu change streams for existing items
    let items = data.lock_ref().items.clone();
    let mut icon_streams = Vec::with_capacity(items.len());
    let mut menu_streams = Vec::with_capacity(items.len());

    for item in &items {
        let (dest, path) = if let Some(idx) = item.name.find('/') {
            (&item.name[..idx], &item.name[idx..])
        } else {
            (item.name.as_str(), "/StatusNotifierItem")
        };

        // Icon pixmap changes
        let item_proxy_result = StatusNotifierItemProxy::builder(conn)
            .destination(dest.to_owned())
            .and_then(|b| b.path(path.to_owned()));

        if let Ok(builder) = item_proxy_result
            && let Ok(proxy) = builder.build().await
        {
            let name = item.name.clone();
            let data_icon = data.clone();

            icon_streams.push(
                proxy
                    .receive_icon_pixmap_changed()
                    .await
                    .filter_map(move |icon_change| {
                        let name = name.clone();
                        let data = data_icon.clone();
                        async move {
                            if let Ok(icons) = icon_change.get().await {
                                let icons: Vec<dbus::IconPixmap> = icons;
                                if let Some(mut icon) =
                                    icons.into_iter().max_by_key(|i| (i.width, i.height))
                                {
                                    for pixel in icon.bytes.chunks_exact_mut(4) {
                                        pixel.rotate_left(1);
                                    }
                                    let tray_icon = TrayIcon::Pixmap {
                                        width: icon.width as u32,
                                        height: icon.height as u32,
                                        data: Arc::new(icon.bytes),
                                    };

                                    let mut guard = data.lock_mut();
                                    if let Some(item) =
                                        guard.items.iter_mut().find(|i| i.name == name)
                                    {
                                        item.icon = Some(tray_icon);
                                    }
                                }
                            }
                            Some(())
                        }
                    })
                    .boxed(),
            );
        }

        // Menu layout changes
        if !item.menu_path.is_empty() && item.menu_path != "/" {
            let menu_proxy_result = DBusMenuProxy::builder(conn)
                .destination(item.dest.clone())
                .and_then(|b| b.path(item.menu_path.clone()));

            if let Ok(builder) = menu_proxy_result
                && let Ok(proxy) = builder.build().await
                && let Ok(layout_stream) = proxy.receive_layout_updated().await
            {
                let name = item.name.clone();
                let data_menu = data.clone();
                let proxy_clone = proxy.clone();

                menu_streams.push(
                    layout_stream
                        .filter_map(move |_| {
                            let name = name.clone();
                            let data = data_menu.clone();
                            let proxy = proxy_clone.clone();
                            async move {
                                if let Ok((_, layout)) = proxy.get_layout(0, -1, &[]).await {
                                    let mut guard = data.lock_mut();
                                    if let Some(item) =
                                        guard.items.iter_mut().find(|i| i.name == name)
                                    {
                                        item.menu = Some(layout);
                                    }
                                }
                                Some(())
                            }
                        })
                        .boxed(),
                );
            }
        }
    }

    // Combine all streams
    let mut events = select_all(vec![registered, unregistered]);
    for stream in icon_streams {
        events.push(stream);
    }
    for stream in menu_streams {
        events.push(stream);
    }

    // Process events until stream ends (which shouldn't happen normally)
    while (events.next().await).is_some() {}

    Ok(())
}

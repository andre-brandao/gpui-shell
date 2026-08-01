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
use zbus::proxy::CacheProperties;

use crate::lifecycle::{Lifecycle, ManagedService, RunToken};

/// Interface the item proxies below talk to.
const ITEM_INTERFACE: &str = "org.kde.StatusNotifierItem";

/// Properties whose change means the icon has to be re-read.
const ICON_PROPERTIES: [&str; 2] = ["IconPixmap", "IconName"];

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
    /// Menu layout. Behind an `Arc` because the bar clones every item it draws
    /// on every repaint, and a menu tree is the expensive part of an item.
    pub menu: Option<Arc<MenuLayout>>,
    /// Internal: D-Bus destination for commands.
    dest: String,
    /// Internal: Menu path.
    menu_path: String,
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
    /// Left-click activation (for apps without menus, e.g. "show window").
    Activate { item_name: String },
    /// Middle-click activation.
    SecondaryActivate { item_name: String },
    /// Right-click context menu.
    ContextMenu { item_name: String },
    /// Notify app that a submenu is about to be shown (triggers lazy population).
    AboutToShow { item_name: String, menu_id: i32 },
}

/// Event-driven system tray subscriber.
#[derive(Debug, Clone, Default)]
pub struct TraySubscriber {
    data: Mutable<TrayData>,
    lifecycle: Lifecycle,
}

impl TraySubscriber {
    /// Create a new tray subscriber. The StatusNotifierWatcher server and
    /// the event listener start with [`ManagedService::start`].
    pub fn new() -> Self {
        Self::default()
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
        let conn = crate::bus::session().await?;

        match command {
            TrayCommand::MenuItemClicked { item_name, menu_id } => {
                let data = self.data.lock_ref();
                if let Some(item) = data.items.iter().find(|i| i.name == item_name) {
                    let menu_proxy = DBusMenuProxy::builder(&conn)
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
                            item.menu = Some(Arc::new(new_layout));
                        }
                    }
                }
            }
            TrayCommand::Activate { item_name } => {
                if let Ok(proxy) = item_proxy(&conn, &item_name).await {
                    debug!("Activating tray item {}", item_name);
                    let _ = proxy.activate(0, 0).await;
                }
            }
            TrayCommand::SecondaryActivate { item_name } => {
                if let Ok(proxy) = item_proxy(&conn, &item_name).await {
                    debug!("Secondary-activating tray item {}", item_name);
                    let _ = proxy.secondary_activate(0, 0).await;
                }
            }
            TrayCommand::ContextMenu { item_name } => {
                if let Ok(proxy) = item_proxy(&conn, &item_name).await {
                    debug!("Context menu for tray item {}", item_name);
                    let _ = proxy.context_menu(0, 0).await;
                }
            }
            TrayCommand::AboutToShow { item_name, menu_id } => {
                let data = self.data.lock_ref();
                if let Some(item) = data.items.iter().find(|i| i.name == item_name)
                    && !item.menu_path.is_empty()
                    && item.menu_path != "/"
                {
                    let menu_proxy = DBusMenuProxy::builder(&conn)
                        .destination(item.dest.clone())?
                        .path(item.menu_path.clone())?
                        .build()
                        .await?;

                    debug!("about_to_show({}) for {}", menu_id, item_name);
                    let needs_update = menu_proxy.about_to_show(menu_id).await.unwrap_or(false);

                    if needs_update {
                        drop(data);
                        if let Ok((_, new_layout)) = menu_proxy.get_layout(0, -1, &[]).await {
                            let mut data = self.data.lock_mut();
                            if let Some(item) = data.items.iter_mut().find(|i| i.name == item_name)
                            {
                                item.menu = Some(Arc::new(new_layout));
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

/// Split an item name into its D-Bus destination and object path.
fn split_item_name(name: &str) -> (&str, &str) {
    match name.find('/') {
        Some(idx) => (&name[..idx], &name[idx..]),
        None => (name, "/StatusNotifierItem"),
    }
}

/// Build a StatusNotifierItem proxy for the given item name.
///
/// Property caching is off on purpose. With it on, zbus calls `GetAll` on the
/// first property read and keeps every property as an `OwnedValue` for as long
/// as the proxy lives - and zvariant stores a byte array as one `Value` per
/// byte, so an item's `IconPixmap` costs megabytes in that form. The proxies
/// the listener holds live for the whole session, so that memory never came
/// back. Reads are plain `Get` calls now: the shell asks for an icon rarely,
/// and it keeps only the decoded pixels it actually draws.
async fn item_proxy(
    conn: &zbus::Connection,
    item_name: &str,
) -> anyhow::Result<StatusNotifierItemProxy<'static>> {
    let (dest, path) = split_item_name(item_name);

    Ok(StatusNotifierItemProxy::builder(conn)
        .destination(dest.to_owned())?
        .path(path.to_owned())?
        .cache_properties(CacheProperties::No)
        .build()
        .await?)
}

/// Read an item's icon: the largest pixmap it offers, else its themed icon name.
async fn fetch_icon(proxy: &StatusNotifierItemProxy<'_>) -> Option<TrayIcon> {
    let pixmap = proxy.icon_pixmap().await.ok().and_then(|icons| {
        icons
            .into_iter()
            .filter(|i| i.width > 0 && i.height > 0 && !i.bytes.is_empty())
            .max_by_key(|i| (i.width, i.height))
            .map(|mut i| {
                // Convert ARGB to RGBA
                for pixel in i.bytes.as_chunks_mut::<4>().0 {
                    pixel.rotate_left(1);
                }
                TrayIcon::Pixmap {
                    width: i.width as u32,
                    height: i.height as u32,
                    data: Arc::new(i.bytes),
                }
            })
    });

    // Apps that offer no pixmap - the property errors, or is an empty array -
    // name a themed icon instead.
    match pixmap {
        Some(icon) => Some(icon),
        None => proxy
            .icon_name()
            .await
            .ok()
            .filter(|n| !n.is_empty())
            .map(TrayIcon::Name),
    }
}

/// Whether two icons carry the same picture.
fn same_icon(a: &TrayIcon, b: &TrayIcon) -> bool {
    match (a, b) {
        (TrayIcon::Name(a), TrayIcon::Name(b)) => a == b,
        (
            TrayIcon::Pixmap {
                width: aw,
                height: ah,
                data: ad,
            },
            TrayIcon::Pixmap {
                width: bw,
                height: bh,
                data: bd,
            },
        ) => aw == bw && ah == bh && (Arc::ptr_eq(ad, bd) || ad == bd),
        _ => false,
    }
}

/// Re-read one item's icon and store it.
async fn refresh_icon(
    proxy: &StatusNotifierItemProxy<'_>,
    item_name: &str,
    data: &Mutable<TrayData>,
) {
    let Some(icon) = fetch_icon(proxy).await else {
        return;
    };

    // Apps announce an icon change twice as often as not (NewIcon *and*
    // PropertiesChanged), and dropping a `lock_mut` guard wakes every
    // subscriber whether or not anything changed - which costs a full clone of
    // the tray state and a repaint. So compare under a read lock first, and let
    // it go before taking the write lock: holding both deadlocks.
    {
        let guard = data.lock_ref();
        match guard.items.iter().find(|i| i.name == item_name) {
            Some(item) if item.icon.as_ref().is_some_and(|old| same_icon(old, &icon)) => return,
            Some(_) => {}
            None => return,
        }
    }

    let mut guard = data.lock_mut();
    if let Some(item) = guard.items.iter_mut().find(|i| i.name == item_name) {
        item.icon = Some(icon);
    }
}

/// Whether a PropertiesChanged signal touches the item's icon.
fn changes_icon(change: &zbus::fdo::PropertiesChanged) -> bool {
    let Ok(args) = change.args() else {
        return false;
    };

    args.interface_name == ITEM_INTERFACE
        && ICON_PROPERTIES.iter().any(|prop| {
            args.changed_properties.contains_key(prop) || args.invalidated_properties.contains(prop)
        })
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
    let (dest, _) = split_item_name(name);
    let item_proxy = item_proxy(conn, name).await?;

    let icon = fetch_icon(&item_proxy).await;
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

        menu_proxy
            .get_layout(0, -1, &[])
            .await
            .ok()
            .map(|(_, l)| Arc::new(l))
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

impl ManagedService for TraySubscriber {
    fn name(&self) -> &'static str {
        "Tray"
    }

    fn lifecycle(&self) -> &Lifecycle {
        &self.lifecycle
    }

    /// Tray icons are RGBA pixmaps, usually the largest state the shell
    /// holds, so they are counted byte for byte.
    fn memory_bytes(&self) -> usize {
        let data = self.data.lock_ref();
        size_of::<TrayData>()
            + data
                .items
                .iter()
                .map(|item| {
                    let icon = match &item.icon {
                        Some(TrayIcon::Name(name)) => name.len(),
                        Some(TrayIcon::Pixmap { data, .. }) => data.len(),
                        None => 0,
                    };
                    let menu = item.menu.as_deref().map_or(0, menu_layout_bytes);
                    size_of::<TrayItem>()
                        + item.name.len()
                        + item.title.as_ref().map_or(0, String::len)
                        + item.id.as_ref().map_or(0, String::len)
                        + icon
                        + menu
                })
                .sum::<usize>()
    }

    fn start(&self) {
        let token = self.lifecycle.begin();
        let data = self.data.clone();

        tokio::spawn(async move {
            let conn = match StatusNotifierWatcher::start_server().await {
                Ok(conn) => conn,
                Err(e) => {
                    error!("Failed to start the StatusNotifierWatcher server: {}", e);
                    token.error(e.to_string());
                    return;
                }
            };

            match fetch_tray_data(&conn).await {
                Ok(fetched) => {
                    info!("Tray service started with {} items", fetched.items.len());
                    data.set(fetched);
                    token.active();
                }
                Err(e) => {
                    error!("Failed to fetch initial tray data: {}", e);
                    token.error(e.to_string());
                }
            }

            start_listener(data, token, conn);
        });
    }
}

/// Retained size of a menu layout, including its submenus.
fn menu_layout_bytes(layout: &MenuLayout) -> usize {
    let props = &layout.1;
    size_of::<MenuLayout>()
        + [
            &props.children_display,
            &props.label,
            &props.type_,
            &props.toggle_type,
            &props.icon_name,
        ]
        .iter()
        .map(|field| field.as_ref().map_or(0, String::len))
        .sum::<usize>()
        + layout.2.iter().map(menu_layout_bytes).sum::<usize>()
}

/// Start the D-Bus listener in a dedicated thread.
fn start_listener(data: Mutable<TrayData>, token: RunToken, conn: zbus::Connection) {
    thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                error!("Failed to create Tokio runtime for Tray listener: {}", e);
                token.error(e.to_string());
                return;
            }
        };

        rt.block_on(async move {
            while token.alive() {
                if let Err(e) = run_listener(&data, &conn, &token).await {
                    error!("Tray listener error: {}", e);
                    token.error(e.to_string());
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
            }
        });
    });
}

/// Run the tray event listener.
async fn run_listener(
    data: &Mutable<TrayData>,
    conn: &zbus::Connection,
    token: &RunToken,
) -> anyhow::Result<()> {
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
        let (dest, path) = split_item_name(&item.name);

        // Icon changes. Apps announce them with the NewIcon signal, with a
        // PropertiesChanged on IconPixmap/IconName, or both, so watch the two
        // and re-read the icon when either fires.
        if let Ok(proxy) = item_proxy(conn, &item.name).await {
            if let Ok(new_icon) = proxy.receive_new_icon().await {
                let name = item.name.clone();
                let data_icon = data.clone();
                let icon_proxy = proxy.clone();
                icon_streams.push(
                    new_icon
                        .filter_map(move |_| {
                            let name = name.clone();
                            let data = data_icon.clone();
                            let proxy = icon_proxy.clone();
                            async move {
                                refresh_icon(&proxy, &name, &data).await;
                                Some(())
                            }
                        })
                        .boxed(),
                );
            }

            let properties = zbus::fdo::PropertiesProxy::builder(conn)
                .destination(dest.to_owned())
                .and_then(|b| b.path(path.to_owned()));

            if let Ok(builder) = properties
                && let Ok(properties) = builder.build().await
                && let Ok(changes) = properties.receive_properties_changed().await
            {
                let name = item.name.clone();
                let data_icon = data.clone();
                icon_streams.push(
                    changes
                        .filter_map(move |change| {
                            let name = name.clone();
                            let data = data_icon.clone();
                            let proxy = proxy.clone();
                            async move {
                                if changes_icon(&change) {
                                    refresh_icon(&proxy, &name, &data).await;
                                }
                                Some(())
                            }
                        })
                        .boxed(),
                );
            }
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
                                        item.menu = Some(Arc::new(layout));
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
    while token.alive() && (events.next().await).is_some() {}

    Ok(())
}

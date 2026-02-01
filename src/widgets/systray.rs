use gpui::{
    AnyElement, App, Bounds, Context, MouseButton, Point, Size, Window, WindowBounds, WindowKind,
    WindowOptions, div, prelude::*, px, rgba, white,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, mpsc};
use system_tray::client::{ActivateRequest, Client, Event, UpdateEvent};
use system_tray::item::StatusNotifierItem;
use system_tray::menu::{MenuItem, TrayMenu};

#[derive(Clone)]
struct TrayItem {
    id: String,
    address: String,
    title: String,
    icon_name: Option<String>,
    menu_path: Option<String>,
    menu: Option<TrayMenu>,
}

pub struct Systray {
    items: HashMap<String, TrayItem>,
    client: Arc<Mutex<Option<Client>>>,
}

impl Systray {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let (tx, rx) = mpsc::channel::<SystrayEvent>();
        let client_holder: Arc<Mutex<Option<Client>>> = Arc::new(Mutex::new(None));
        let client_for_thread = client_holder.clone();

        // Spawn tokio runtime in a separate thread for system-tray client
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async move {
                let client = match Client::new().await {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Failed to create systray client: {}", e);
                        return;
                    }
                };

                let mut tray_rx = client.subscribe();

                // Send initial items
                if let Ok(items) = client.items().lock() {
                    for (id, (item, menu)) in items.iter() {
                        let _ = tx.send(SystrayEvent::Add(id.clone(), item.clone(), menu.clone()));
                    }
                }

                // Store client for activation requests
                if let Ok(mut holder) = client_for_thread.lock() {
                    *holder = Some(client);
                }

                // Listen for updates
                while let Ok(event) = tray_rx.recv().await {
                    match event {
                        Event::Add(id, item) => {
                            let _ = tx.send(SystrayEvent::Add(id, *item, None));
                        }
                        Event::Update(id, update) => {
                            let _ = tx.send(SystrayEvent::Update(id, update));
                        }
                        Event::Remove(id) => {
                            let _ = tx.send(SystrayEvent::Remove(id));
                        }
                    }
                }
            });
        });

        // Poll for updates from the systray thread
        cx.spawn(async move |this, cx| {
            loop {
                let mut updated = false;

                while let Ok(event) = rx.try_recv() {
                    updated = true;
                    let _ = this.update(cx, |this, _| match event {
                        SystrayEvent::Add(id, item, menu) => {
                            let tray_item = TrayItem {
                                id: id.clone(),
                                address: item.id.clone(),
                                title: item.title.clone().unwrap_or_default(),
                                icon_name: item.icon_name.clone(),
                                menu_path: item.menu.clone(),
                                menu,
                            };
                            this.items.insert(id, tray_item);
                        }
                        SystrayEvent::Update(id, update) => {
                            if let Some(tray_item) = this.items.get_mut(&id) {
                                match update {
                                    UpdateEvent::Title(title) => {
                                        tray_item.title = title.unwrap_or_default();
                                    }
                                    UpdateEvent::Icon { icon_name, .. } => {
                                        tray_item.icon_name = icon_name;
                                    }
                                    UpdateEvent::Menu(menu) => {
                                        tray_item.menu = Some(menu);
                                    }
                                    _ => {}
                                }
                            }
                        }
                        SystrayEvent::Remove(id) => {
                            this.items.remove(&id);
                        }
                    });
                }

                if updated {
                    let _ = this.update(cx, |_, cx| cx.notify());
                }

                cx.background_executor()
                    .timer(std::time::Duration::from_millis(100))
                    .await;
            }
        })
        .detach();

        Systray {
            items: HashMap::new(),
            client: client_holder,
        }
    }

    fn open_menu(&self, item: &TrayItem, cx: &mut App) {
        let Some(menu) = item.menu.clone() else {
            return;
        };

        let address = item.address.clone();
        let menu_path = item.menu_path.clone().unwrap_or_default();
        let client = self.client.clone();

        let menu_height = (menu.submenus.len() * 32).min(400) as f32;

        cx.open_window(
            WindowOptions {
                titlebar: None,
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: Point::new(px(1600.), px(32.)),
                    size: Size::new(px(220.), px(menu_height)),
                })),
                kind: WindowKind::PopUp,
                is_movable: false,
                focus: true,
                ..Default::default()
            },
            |_, cx| {
                cx.new(|_| SystrayMenu {
                    menu,
                    address,
                    menu_path,
                    client,
                })
            },
        )
        .ok();
    }

    fn render_tray_item(&self, item: TrayItem, cx: &mut Context<Self>) -> AnyElement {
        let item_clone = item.clone();

        div()
            .id(item.id.clone())
            .px(px(6.))
            .py(px(4.))
            .rounded(px(4.))
            .cursor_pointer()
            .hover(|s| s.bg(rgba(0x333333ff)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| {
                    this.open_menu(&item_clone, cx);
                }),
            )
            .child(
                item.icon_name
                    .as_ref()
                    .and_then(|n| n.chars().next())
                    .or_else(|| item.title.chars().next())
                    .unwrap_or('?')
                    .to_string(),
            )
            .into_any_element()
    }
}

enum SystrayEvent {
    Add(String, StatusNotifierItem, Option<TrayMenu>),
    Update(String, UpdateEvent),
    Remove(String),
}

impl Render for Systray {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let items: Vec<TrayItem> = self.items.values().cloned().collect();

        let rendered_items: Vec<AnyElement> = items
            .into_iter()
            .map(|item| self.render_tray_item(item, cx))
            .collect();

        div()
            .flex()
            .items_center()
            .gap(px(4.))
            .children(rendered_items)
    }
}

// Separate menu window component
struct SystrayMenu {
    menu: TrayMenu,
    address: String,
    menu_path: String,
    client: Arc<Mutex<Option<Client>>>,
}

impl SystrayMenu {
    fn activate_menu_item(&self, submenu_id: i32, cx: &mut Context<Self>) {
        let client = self.client.clone();
        let address = self.address.clone();
        let menu_path = self.menu_path.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async move {
                if let Ok(holder) = client.lock() {
                    if let Some(client) = holder.as_ref() {
                        let request = ActivateRequest::MenuItem {
                            address,
                            menu_path,
                            submenu_id,
                        };
                        let _ = client.activate(request).await;
                    }
                }
            });
        });

        // Close the menu window by notifying (window will close on click outside)
        cx.notify();
    }

    fn render_menu_item(&self, item: &MenuItem, cx: &mut Context<Self>) -> AnyElement {
        if !item.visible {
            return div().into_any_element();
        }

        let label = item
            .label
            .as_ref()
            .map(|l| l.replace('_', ""))
            .unwrap_or_default();

        if label.is_empty() {
            // Separator
            return div()
                .h(px(1.))
                .w_full()
                .bg(rgba(0x444444ff))
                .my(px(4.))
                .into_any_element();
        }

        let submenu_id = item.id;
        let enabled = item.enabled;

        div()
            .id(format!("menu-item-{}", submenu_id))
            .w_full()
            .px(px(12.))
            .py(px(8.))
            .cursor_pointer()
            .when(!enabled, |s| s.opacity(0.5))
            .hover(|s| s.bg(rgba(0x3b82f6ff)))
            .when(enabled, |el| {
                el.on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _, _, cx| {
                        this.activate_menu_item(submenu_id, cx);
                    }),
                )
            })
            .child(label)
            .into_any_element()
    }
}

impl Render for SystrayMenu {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let menu_items: Vec<AnyElement> = self
            .menu
            .submenus
            .iter()
            .map(|item| self.render_menu_item(item, cx))
            .collect();

        div()
            .size_full()
            .bg(rgba(0x1a1a1aff))
            .border_1()
            .border_color(rgba(0x333333ff))
            .rounded(px(8.))
            .text_color(white())
            // .overflow_scroll()
            .children(menu_items)
    }
}

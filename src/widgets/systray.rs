use crate::ui::{PanelConfig, toggle_panel};
use gpui::{
    AnyElement, App, Context, MouseButton, Window, div, layer_shell::Anchor, prelude::*, px, rgba,
    white,
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
                                id: item.id.clone(),
                                address: id.clone(), // DBus bus name
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

    fn toggle_menu(&mut self, item: &TrayItem, cx: &mut App) {
        let Some(menu) = item.menu.clone() else {
            return;
        };

        let address = item.address.clone();
        let menu_path = item.menu_path.clone().unwrap_or_default();
        let client = self.client.clone();
        let panel_id = format!("systray-{}", address);

        // Calculate menu height based on visible items
        let visible_items = count_visible_items(&menu.submenus);
        let menu_height = (visible_items * 32).min(500) as f32 + 16.0;

        let config = PanelConfig {
            width: 250.0,
            height: menu_height,
            anchor: Anchor::TOP | Anchor::RIGHT,
            margin: (0.0, 8.0, 0.0, 0.0),
            namespace: "systray-menu".to_string(),
        };

        toggle_panel(&panel_id, config, cx, move |_cx| SystrayMenuContent {
            menu,
            address,
            menu_path,
            client,
        });
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
                    this.toggle_menu(&item_clone, cx);
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

fn count_visible_items(items: &[MenuItem]) -> usize {
    items
        .iter()
        .filter(|i| i.visible)
        .map(|i| 1 + count_visible_items(&i.submenu))
        .sum()
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

/// Systray menu content widget - displays the menu items.
pub struct SystrayMenuContent {
    menu: TrayMenu,
    address: String,
    menu_path: String,
    client: Arc<Mutex<Option<Client>>>,
}

impl SystrayMenuContent {
    fn close_window(&mut self, window: &mut Window) {
        window.remove_window();
    }

    fn activate_menu_item(&mut self, submenu_id: i32, window: &mut Window) {
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

        // Close the menu window
        self.close_window(window);
    }

    fn render_menu_items(
        &self,
        items: &[MenuItem],
        depth: usize,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let mut elements = Vec::new();

        for item in items {
            if !item.visible {
                continue;
            }

            let label = item
                .label
                .as_ref()
                .map(|l| l.replace('_', ""))
                .unwrap_or_default();

            if label.is_empty() && item.submenu.is_empty() {
                // Separator
                elements.push(
                    div()
                        .h(px(1.))
                        .w_full()
                        .bg(rgba(0x444444ff))
                        .my(px(4.))
                        .into_any_element(),
                );
                continue;
            }

            let submenu_id = item.id;
            let enabled = item.enabled;
            let has_submenu = !item.submenu.is_empty();
            let indent = depth * 16;

            // Render the item
            elements.push(
                div()
                    .id(format!("menu-item-{}", submenu_id))
                    .w_full()
                    .pl(px(12.0 + indent as f32))
                    .pr(px(12.))
                    .py(px(6.))
                    .cursor_pointer()
                    .when(!enabled, |s| s.opacity(0.5))
                    .hover(|s| s.bg(rgba(0x3b82f6ff)))
                    .when(enabled && !has_submenu, |el| {
                        el.on_click(cx.listener(move |this, _, window, _cx| {
                            this.activate_menu_item(submenu_id, window);
                        }))
                    })
                    .child(
                        div()
                            .flex()
                            .w_full()
                            .justify_between()
                            .child(label)
                            .when(has_submenu, |el| el.child("▸")),
                    )
                    .into_any_element(),
            );

            // Render submenu items inline (expanded)
            if has_submenu {
                let submenu_elements = self.render_menu_items(&item.submenu, depth + 1, cx);
                elements.extend(submenu_elements);
            }
        }

        elements
    }
}

impl Render for SystrayMenuContent {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let menu_items = self.render_menu_items(&self.menu.submenus, 0, cx);

        div()
            .id("systray-menu")
            .size_full()
            .bg(rgba(0x1a1a1aee))
            .border_1()
            .border_color(rgba(0x333333ff))
            .rounded(px(12.))
            .py(px(8.))
            .text_color(white())
            .overflow_hidden()
            .children(menu_items)
    }
}

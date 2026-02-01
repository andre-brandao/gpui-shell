use gpui::{Context, Window, div, prelude::*, px};
use std::collections::HashMap;
use std::sync::mpsc;
use system_tray::client::{Client, Event, UpdateEvent};
use system_tray::item::StatusNotifierItem;

#[derive(Clone)]
struct TrayItem {
    id: String,
    title: String,
}

pub struct Systray {
    items: HashMap<String, TrayItem>,
}

impl Systray {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let (tx, rx) = mpsc::channel::<SystrayEvent>();

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
                    for (id, (item, _)) in items.iter() {
                        let _ = tx.send(SystrayEvent::Add(id.clone(), item.clone()));
                    }
                }

                // Listen for updates
                while let Ok(event) = tray_rx.recv().await {
                    match event {
                        Event::Add(id, item) => {
                            let _ = tx.send(SystrayEvent::Add(id, *item));
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
                        SystrayEvent::Add(id, item) => {
                            let tray_item = TrayItem {
                                id: id.clone(),
                                title: item.title.clone().unwrap_or_default(),
                            };
                            this.items.insert(id, tray_item);
                        }
                        SystrayEvent::Update(id, update) => {
                            if let Some(tray_item) = this.items.get_mut(&id) {
                                if let UpdateEvent::Title(title) = update {
                                    tray_item.title = title.unwrap_or_default();
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
        }
    }
}

enum SystrayEvent {
    Add(String, StatusNotifierItem),
    Update(String, UpdateEvent),
    Remove(String),
}

impl Render for Systray {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap(px(8.))
            .children(self.items.values().map(|item| {
                div()
                    .id(item.id.clone())
                    .size(px(20.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(item.title.chars().next().unwrap_or('?').to_string())
            }))
    }
}

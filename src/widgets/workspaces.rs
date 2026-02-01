use gpui::{Context, MouseButton, Window, div, prelude::*, px, rgba};
use hyprland::data::{Workspace, Workspaces};
use hyprland::dispatch::{Dispatch, DispatchType, WorkspaceIdentifierWithSpecial};
use hyprland::event_listener::EventListener;
use hyprland::prelude::*;
use std::sync::mpsc;

#[derive(Clone, Debug)]
struct WorkspaceInfo {
    id: i32,
    name: String,
    is_active: bool,
}

pub struct HyprlandWorkspaces {
    workspaces: Vec<WorkspaceInfo>,
}

impl HyprlandWorkspaces {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let (tx, rx) = mpsc::channel::<()>();

        // Spawn the blocking listener in a separate thread
        std::thread::spawn(move || {
            let mut listener = EventListener::new();

            let tx_changed = tx.clone();
            listener.add_workspace_changed_handler(move |_| {
                let _ = tx_changed.send(());
            });

            let tx_added = tx.clone();
            listener.add_workspace_added_handler(move |_| {
                let _ = tx_added.send(());
            });

            let tx_deleted = tx.clone();
            listener.add_workspace_deleted_handler(move |_| {
                let _ = tx_deleted.send(());
            });

            println!("Hyprland workspace listener started.");
            let _ = listener.start_listener();
        });

        // Poll the channel for updates
        cx.spawn(async move |this, cx| {
            loop {
                // Check if there are any events (non-blocking)
                let has_update = rx.try_recv().is_ok();

                if has_update {
                    // Drain any additional queued events
                    while rx.try_recv().is_ok() {}

                    let workspaces = Self::fetch_workspaces();
                    let _ = this.update(cx, |this, cx| {
                        this.workspaces = workspaces;
                        cx.notify();
                    });
                }

                cx.background_executor()
                    .timer(std::time::Duration::from_millis(50))
                    .await;
            }
        })
        .detach();

        HyprlandWorkspaces {
            workspaces: Self::fetch_workspaces(),
        }
    }

    fn fetch_workspaces() -> Vec<WorkspaceInfo> {
        let active_id = Workspace::get_active().map(|ws| ws.id).unwrap_or(-1);

        Workspaces::get()
            .map(|workspaces| {
                let mut ws_list: Vec<WorkspaceInfo> = workspaces
                    .iter()
                    .map(|ws| WorkspaceInfo {
                        id: ws.id,
                        name: ws.name.clone(),
                        is_active: ws.id == active_id,
                    })
                    .collect();
                ws_list.sort_by_key(|ws| ws.id);
                ws_list
            })
            .unwrap_or_default()
    }
}

impl HyprlandWorkspaces {
    fn switch_to_workspace(id: i32) {
        let _ = Dispatch::call(DispatchType::Workspace(WorkspaceIdentifierWithSpecial::Id(
            id,
        )));
    }
}

impl Render for HyprlandWorkspaces {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().flex().items_center().gap(px(4.)).children(
            self.workspaces
                .iter()
                .filter(|ws| !ws.name.starts_with("special"))
                .map(|ws| {
                    let workspace_id = ws.id;
                    div()
                        .id(format!("workspace-{}", ws.id))
                        // .cursor_pointer()
                        .px(if ws.is_active { px(16.) } else { px(8.) })
                        .py(px(2.))
                        .rounded(px(25.))
                        .bg(if ws.is_active {
                            rgba(0x3b82f6ff)
                        } else {
                            rgba(0x333333ff)
                        })
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |_this, _event, _window, _cx| {
                                Self::switch_to_workspace(workspace_id);
                            }),
                        )
                        .child(ws.name.clone())
                }),
        )
    }
}

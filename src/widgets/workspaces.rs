use gpui::{div, prelude::*, px, rgba};
use hyprland::data::{Workspace, Workspaces};
use hyprland::prelude::*;

#[derive(Clone, Debug)]
pub struct WorkspaceInfo {
    pub id: i32,
    pub name: String,
    pub is_active: bool,
    pub window_count: u16,
}

pub fn fetch_workspaces() -> Vec<WorkspaceInfo> {
    let active_id = Workspace::get_active().map(|ws| ws.id).unwrap_or(-1);

    Workspaces::get()
        .map(|workspaces| {
            workspaces
                .iter()
                .map(|ws| WorkspaceInfo {
                    id: ws.id,
                    name: ws.name.clone(),
                    is_active: ws.id == active_id,
                    window_count: ws.windows,
                })
                .filter(|ws| !ws.name.starts_with("special"))
                .collect()
        })
        .unwrap_or_default()
}

pub fn workspaces(workspaces: &[WorkspaceInfo]) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(4.))
        .children(workspaces.iter().map(|ws| {
            div()
                .px(px(8.))
                .py(px(2.))
                .rounded(px(4.))
                .bg(if ws.is_active {
                    rgba(0x3b82f6ff)
                } else {
                    rgba(0x333333ff)
                })
                .child(ws.name.clone())
        }))
}

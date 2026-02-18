//! Workspaces view for switching between compositor workspaces.

pub mod config;

use gpui::{div, prelude::*, px, AnyElement, App};
use services::CompositorCommand;
use ui::{ActiveTheme, Color, Label, LabelCommon, LabelSize, ListItem, ListItemSpacing};

use self::config::WorkspacesConfig;
use crate::launcher::view::{render_footer_hints, LauncherView, ViewContext};
use crate::state::AppState;

/// Workspaces view - displays and switches between workspaces.
pub struct WorkspacesView {
    prefix: String,
}

impl WorkspacesView {
    pub fn new(config: &WorkspacesConfig) -> Self {
        Self {
            prefix: config.prefix.clone(),
        }
    }

    fn filtered_workspaces(
        &self,
        vx: &ViewContext,
        cx: &App,
    ) -> Vec<services::compositor::Workspace> {
        let compositor = AppState::compositor(cx).get();
        let query_lower = vx.query.to_lowercase();
        compositor
            .workspaces
            .into_iter()
            .filter(|ws| !ws.is_special)
            .filter(|ws| {
                if vx.query.is_empty() {
                    return true;
                }
                let title = if ws.name.is_empty() {
                    format!("Workspace {}", ws.id)
                } else {
                    ws.name.clone()
                };
                title.to_lowercase().contains(&query_lower)
                    || ws.monitor.to_lowercase().contains(&query_lower)
            })
            .collect()
    }
}

impl LauncherView for WorkspacesView {
    fn prefix(&self) -> &str {
        &self.prefix
    }

    fn name(&self) -> &'static str {
        "Workspaces"
    }

    fn icon(&self) -> &'static str {
        "󰍉"
    }

    fn description(&self) -> &'static str {
        "Switch between workspaces"
    }

    fn match_count(&self, vx: &ViewContext, cx: &App) -> usize {
        self.filtered_workspaces(vx, cx).len()
    }

    fn render_item(&self, index: usize, selected: bool, vx: &ViewContext, cx: &App) -> AnyElement {
        let filtered = self.filtered_workspaces(vx, cx);
        let Some(ws) = filtered.get(index) else {
            return div().into_any_element();
        };

        let theme = cx.theme();
        let title = if ws.name.is_empty() {
            format!("Workspace {}", ws.id)
        } else {
            ws.name.clone()
        };
        let subtitle = format!("{} windows on {}", ws.windows, ws.monitor);
        let ws_id = ws.id;
        let compositor_clone = AppState::compositor(cx).clone();
        let interactive_default = theme.interactive.default;
        let icon = self.icon();

        ListItem::new(format!("ws-{}", ws.id))
            .spacing(ListItemSpacing::Sparse)
            .toggle_state(selected)
            .start_slot(
                div()
                    .w(px(28.))
                    .h(px(28.))
                    .rounded(px(6.))
                    .bg(interactive_default)
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(14.))
                    .child(icon),
            )
            .on_click(move |_, _, _cx| {
                let _ = compositor_clone.dispatch(CompositorCommand::FocusWorkspace(ws_id));
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(1.))
                    .child(Label::new(title).size(LabelSize::Default))
                    .child(
                        Label::new(subtitle)
                            .size(LabelSize::Small)
                            .color(Color::Muted),
                    ),
            )
            .into_any_element()
    }

    fn on_select(&self, index: usize, vx: &ViewContext, cx: &mut App) -> bool {
        let filtered = self.filtered_workspaces(vx, cx);
        if let Some(ws) = filtered.get(index) {
            let _ = AppState::compositor(cx).dispatch(CompositorCommand::FocusWorkspace(ws.id));
            true
        } else {
            false
        }
    }

    fn render_footer_bar(&self, _vx: &ViewContext, cx: &App) -> AnyElement {
        render_footer_hints(vec![("Switch", "Enter"), ("Close", "Esc")], cx)
    }
}

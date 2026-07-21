//! Right-click context menu for a dock item: pin/unpin, new window.

use gpui::{
    Context, FocusHandle, Focusable, MouseButton, Render, Subscription, Window, div, prelude::*, px,
};
use ui::{ActiveTheme, radius, spacing};

use super::toggle_pin;

pub(super) struct DockContextMenu {
    panel_id: String,
    item_key: String,
    is_pinned: bool,
    exec: Option<String>,
    app_name: String,
    icon_path: Option<std::path::PathBuf>,
    focus_handle: FocusHandle,
    focus_out_subscription: Option<Subscription>,
}

impl DockContextMenu {
    pub(super) fn new(
        panel_id: String,
        item_key: String,
        is_pinned: bool,
        exec: Option<String>,
        app_name: String,
        icon_path: Option<std::path::PathBuf>,
        _cx: &mut Context<Self>,
    ) -> Self {
        Self {
            panel_id,
            item_key,
            is_pinned,
            exec,
            app_name,
            icon_path,
            focus_handle: _cx.focus_handle(),
            focus_out_subscription: None,
        }
    }

    fn render_entry(
        &self,
        label: String,
        on_click: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme().clone();
        div()
            .id(gpui::SharedString::from(label.clone()))
            .px(px(spacing::MD))
            .py(px(spacing::SM))
            .rounded(px(radius::SM))
            .cursor_pointer()
            .text_color(theme.text.primary)
            .hover(move |style| style.bg(theme.interactive.hover))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _event, window, cx| on_click(this, window, cx)),
            )
            .child(label)
    }
}

impl Focusable for DockContextMenu {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for DockContextMenu {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.focus_out_subscription.is_none() {
            let panel_id = self.panel_id.clone();
            self.focus_out_subscription =
                Some(
                    cx.on_focus_out(&self.focus_handle, window, move |_, _, window, _| {
                        window.remove_window();
                        crate::panel::forget_panel(&panel_id);
                    }),
                );
        }
        if !self.focus_handle.is_focused(window) {
            self.focus_handle.focus(window, cx);
        }

        let theme = cx.theme().clone();
        let pin_label = if self.is_pinned {
            "Remove from Dock".to_string()
        } else {
            "Pin to Dock".to_string()
        };
        let item_key = self.item_key.clone();
        let exec = self.exec.clone();
        let app_name = self.app_name.clone();
        let icon_path = self.icon_path.clone();

        let mut children = vec![
            self.render_entry(
                pin_label,
                move |this, _window, cx| {
                    toggle_pin(&this.item_key, cx);
                    crate::panel::close_panel(cx);
                },
                cx,
            )
            .into_any_element(),
        ];

        if let Some(exec) = exec {
            children.push(
                self.render_entry(
                    "New Window".to_string(),
                    move |_this, _window, cx| {
                        let app = services::Application {
                            name: app_name.clone(),
                            exec: exec.clone(),
                            icon: None,
                            icon_path: icon_path.clone(),
                            description: None,
                            desktop_file: std::path::PathBuf::from(&item_key),
                            startup_wm_class: None,
                        };
                        app.launch();
                        crate::panel::close_panel(cx);
                    },
                    cx,
                )
                .into_any_element(),
            );
        }

        div()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .p(px(spacing::XS))
            .rounded(px(radius::MD))
            .bg(theme.bg.primary)
            .border_1()
            .border_color(theme.border.subtle)
            .children(children)
    }
}

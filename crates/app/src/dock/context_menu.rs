//! Right-click context menu for a dock item: pin/unpin, new window.

use gpui::{Context, MouseButton, Render, Window, div, prelude::*, px};
use ui::{ActiveTheme, radius, spacing};

use super::toggle_pin;

pub(super) struct DockContextMenu {
    item_key: String,
    is_pinned: bool,
    exec: Option<String>,
    app_name: String,
    icon_path: Option<std::path::PathBuf>,
}

impl DockContextMenu {
    pub(super) fn new(
        item_key: String,
        is_pinned: bool,
        exec: Option<String>,
        app_name: String,
        icon_path: Option<std::path::PathBuf>,
        _cx: &mut Context<Self>,
    ) -> Self {
        Self {
            item_key,
            is_pinned,
            exec,
            app_name,
            icon_path,
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

impl Render for DockContextMenu {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
                move |this, window, cx| {
                    toggle_pin(&this.item_key, cx);
                    window.remove_window();
                },
                cx,
            )
            .into_any_element(),
        ];

        if let Some(exec) = exec {
            children.push(
                self.render_entry(
                    "New Window".to_string(),
                    move |_this, window, _cx| {
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
                        window.remove_window();
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

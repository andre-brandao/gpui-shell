//! Launcher button widget for opening the application launcher.

mod config;
pub use config::LauncherBtnConfig;

use crate::launcher;
use gpui::{AnyElement, Context, MouseButton, Window, div, prelude::*, px};
use ui::ActiveTheme;

use super::{BarWidget, style};
use crate::config::ActiveConfig;

/// A button widget that opens the launcher when clicked.
pub struct LauncherBtn;

const LAUNCHER_ICON: &str = "󰀻";

impl LauncherBtn {
    /// Create a new launcher button.
    pub fn new(_cx: &mut Context<Self>) -> Self {
        LauncherBtn
    }

    fn icon(&self, configured_icon: &str) -> String {
        if configured_icon.trim().is_empty() {
            LAUNCHER_ICON.to_string()
        } else {
            configured_icon.to_string()
        }
    }

    fn render_button_content(
        &self,
        icon: String,
        theme: &ui::Theme,
        is_vertical: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .id("launcher-btn")
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |_, _, _, cx| {
                    launcher::toggle(None, cx);
                }),
            )
            .child(
                div().flex().items_center().justify_center().child(
                    div()
                        .text_size(px(style::icon(is_vertical)))
                        .text_color(theme.colors.text)
                        .child(icon),
                ),
            )
            .into_any_element()
    }
}

impl BarWidget for LauncherBtn {
    fn is_interactive(&self) -> bool {
        true
    }

    fn render_vertical(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        let config = &cx.config().bar.modules.launcher_btn;
        self.render_button_content(self.icon(&config.icon), &theme, true, cx)
    }

    fn render_horizontal(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        let config = &cx.config().bar.modules.launcher_btn;
        self.render_button_content(self.icon(&config.icon), &theme, false, cx)
    }
}

impl Render for LauncherBtn {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_bar_widget(window, cx)
    }
}

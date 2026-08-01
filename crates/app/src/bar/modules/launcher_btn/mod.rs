//! Launcher button widget for opening the application launcher.

mod config;
pub use config::LauncherBtnConfig;

use crate::launcher;
use gpui::{AnyElement, Context, Window, prelude::*};
use ui::{ButtonCommon, ButtonLike, ButtonStyle, Clickable, Color, Icon, IconName, IconSource};

use super::{BarWidget, style};
use crate::config::ActiveConfig;

/// A button widget that opens the launcher when clicked.
pub struct LauncherBtn;

const LAUNCHER_ICON: IconName = IconName::Layers;

impl LauncherBtn {
    /// Create a new launcher button.
    pub fn new(_cx: &mut Context<Self>) -> Self {
        LauncherBtn
    }

    fn render_button_content(
        &self,
        icon: IconSource,
        is_vertical: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        // The chip around this button already paints the hover and owns the
        // padding, so the button itself stays transparent and unpadded.
        ButtonLike::new("launcher-btn")
            .style(ButtonStyle::Transparent)
            .on_click(cx.listener(move |_, _, _, cx| {
                launcher::toggle(None, cx);
            }))
            .child(
                Icon::new(icon)
                    .size(style::icon(is_vertical))
                    .color(Color::Default),
            )
            .into_any_element()
    }
}

impl BarWidget for LauncherBtn {
    fn is_interactive(&self) -> bool {
        true
    }

    fn render_vertical(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let config = &cx.config().bar.modules.launcher_btn;
        let icon = crate::icons::source_or(config.icon.as_ref(), LAUNCHER_ICON);
        self.render_button_content(icon, true, cx)
    }

    fn render_horizontal(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let config = &cx.config().bar.modules.launcher_btn;
        let icon = crate::icons::source_or(config.icon.as_ref(), LAUNCHER_ICON);
        self.render_button_content(icon, false, cx)
    }
}

impl Render for LauncherBtn {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_bar_widget(window, cx)
    }
}

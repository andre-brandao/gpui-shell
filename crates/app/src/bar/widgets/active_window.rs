//! Active window widget displaying the title of the currently focused window.

use futures_signals::signal::SignalExt;
use futures_util::StreamExt;
use gpui::{Context, Render, Window, div, prelude::*, px};
use services::CompositorState;
use ui::{ActiveTheme, radius, spacing};

use super::style;
use crate::config::ActiveConfig;
use crate::state::AppState;

/// Maximum characters to display before truncating the title.
const MAX_TITLE_LENGTH_HORIZONTAL: usize = 64;

/// Widget that displays the currently focused window's title.
pub struct ActiveWindow {
    _compositor: services::CompositorSubscriber,
    state: CompositorState,
}

impl ActiveWindow {
    /// Create a new active window widget.
    pub fn new(cx: &mut Context<Self>) -> Self {
        let compositor = AppState::services(cx).compositor.clone();
        let state = compositor.get();

        // Subscribe to compositor state changes
        cx.spawn({
            let mut signal = compositor.subscribe().to_stream();
            async move |this, cx| {
                while let Some(new_state) = signal.next().await {
                    let result = this.update(cx, |this, cx| {
                        this.state = new_state;
                        cx.notify();
                    });
                    if result.is_err() {
                        break;
                    }
                }
            }
        })
        .detach();

        Self {
            _compositor: compositor,
            state,
        }
    }

    /// Get the display title, truncated if necessary.
    fn display_title(&self) -> String {
        let title = self
            .state
            .active_window
            .as_ref()
            .map(|w| w.title.as_str())
            .unwrap_or("");

        if title.is_empty() {
            return String::new();
        }

        if let Some((cutoff, _)) = title.char_indices().nth(MAX_TITLE_LENGTH_HORIZONTAL) {
            format!("{}…", &title[..cutoff])
        } else {
            title.to_string()
        }
    }

    fn window_icon(&self) -> Option<&'static str> {
        let window = self.state.active_window.as_ref()?;
        let haystack = format!(
            "{} {}",
            window.class.to_lowercase(),
            window.title.to_lowercase()
        );

        if haystack.contains("firefox") {
            Some("󰈹")
        } else if haystack.contains("chrome") || haystack.contains("chromium") {
            Some("")
        } else if haystack.contains("telegram") {
            Some("")
        } else if haystack.contains("discord") || haystack.contains("vesktop") {
            Some("󰙯")
        } else if haystack.contains("spotify") {
            Some("󰓇")
        } else if haystack.contains("steam") {
            Some("󰓓")
        } else if haystack.contains("thunderbird") {
            Some("󰴃")
        } else if haystack.contains("code")
            || haystack.contains("zed")
            || haystack.contains("neovim")
            || haystack.contains("nvim")
        {
            Some("󰨞")
        } else if haystack.contains("kitty")
            || haystack.contains("alacritty")
            || haystack.contains("wezterm")
            || haystack.contains("terminal")
        {
            Some("󰆍")
        } else if haystack.contains("files")
            || haystack.contains("nautilus")
            || haystack.contains("thunar")
            || haystack.contains("dolphin")
        {
            Some("󰉋")
        } else {
            Some("󰣇")
        }
    }

    fn vertical_label(&self) -> String {
        let source = self
            .state
            .active_window
            .as_ref()
            .map(|window| {
                if window.class.trim().is_empty() {
                    window.title.trim()
                } else {
                    window.class.trim()
                }
            })
            .unwrap_or_default();

        let label = source
            .chars()
            .filter(|ch| ch.is_ascii_alphanumeric())
            .take(3)
            .collect::<String>();

        if label.is_empty() {
            String::new()
        } else {
            label.to_uppercase()
        }
    }
}

impl Render for ActiveWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let is_vertical = cx.config().bar.is_vertical();
        let title = self.display_title();
        let icon = self.window_icon();
        let vertical_label = self.vertical_label();

        if is_vertical {
            div()
                .id("active-window")
                .flex()
                .flex_col()
                .items_center()
                .gap(px(style::CHIP_GAP))
                .px(px(style::chip_padding_x(true)))
                .py(px(style::CHIP_PADDING_Y))
                .rounded(px(radius::SM))
                .when_some(icon, |el, icon| {
                    el.child(
                        div()
                            .text_size(px(style::icon(true)))
                            .text_color(theme.text.muted)
                            .child(icon),
                    )
                })
                .when(!vertical_label.is_empty(), |el| {
                    el.child(
                        div()
                            .text_size(px(style::label(true)))
                            .text_color(theme.text.secondary)
                            .child(vertical_label),
                    )
                })
        } else {
            div()
                .id("active-window")
                .flex()
                .items_center()
                .justify_center()
                .gap(px(style::CHIP_GAP))
                .px(px(spacing::SM))
                .py(px(style::CHIP_PADDING_Y))
                .rounded(px(radius::SM))
                .text_size(px(style::label(false)))
                .text_color(theme.text.secondary)
                .overflow_hidden()
                .when_some(icon, |el, icon| {
                    el.child(
                        div()
                            .text_size(px(style::icon(false)))
                            .text_color(theme.text.muted)
                            .child(icon),
                    )
                })
                .child(div().overflow_hidden().text_ellipsis().child(title))
        }
    }
}

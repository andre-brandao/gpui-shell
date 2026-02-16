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

    fn vertical_lines(&self) -> Vec<String> {
        let source = self
            .state
            .active_window
            .as_ref()
            .map(|window| {
                if window.title.trim().is_empty() {
                    window.class.trim()
                } else {
                    window.title.trim()
                }
            })
            .unwrap_or_default();

        if source.is_empty() {
            return Vec::new();
        }

        let token = source
            .split(|ch: char| !ch.is_alphanumeric())
            .filter(|part| !part.is_empty())
            .find(|part| {
                let lower = part.to_lowercase();
                !matches!(lower.as_str(), "org" | "com" | "io" | "app" | "www")
            })
            .unwrap_or(source);

        let compact = token
            .chars()
            .filter(|ch| ch.is_alphanumeric())
            .take(4)
            .collect::<String>()
            .to_uppercase();

        if compact.is_empty() {
            return Vec::new();
        }

        let mut lines = Vec::new();
        let first = compact.chars().take(2).collect::<String>();
        let second = compact.chars().skip(2).take(2).collect::<String>();

        lines.push(first);
        if !second.is_empty() {
            lines.push(second);
        }

        lines
    }
}

impl Render for ActiveWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let is_vertical = cx.config().bar.is_vertical();
        let title = self.display_title();
        let icon = self.window_icon();
        let vertical_lines = self.vertical_lines();
        let has_vertical_lines = !vertical_lines.is_empty();
        let interactive_default = theme.interactive.default;
        let border_subtle = theme.border.subtle;
        let text_primary = theme.text.primary;
        let text_secondary = theme.text.secondary;
        let text_muted = theme.text.muted;

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
                .children(
                    vertical_lines
                        .into_iter()
                        .enumerate()
                        .map(move |(idx, line)| {
                            div()
                                .text_size(px(style::label(true)))
                                .text_color(if idx == 0 {
                                    text_primary
                                } else {
                                    text_secondary
                                })
                                .child(line)
                        }),
                )
                .when(icon.is_some() && !has_vertical_lines, |el| {
                    el.when_some(icon, |el, icon| {
                        el.child(
                            div()
                                .text_size(px(style::icon(true)))
                                .text_color(text_muted)
                                .child(icon),
                        )
                    })
                })
                .when(icon.is_none() && !has_vertical_lines, |el| {
                    el.child(
                        div()
                            .text_size(px(style::label(true)))
                            .text_color(text_muted)
                            .child("APP"),
                    )
                })
        } else {
            div()
                .id("active-window")
                .flex()
                .items_center()
                .justify_center()
                .gap(px(style::CHIP_GAP))
                .px(px(spacing::MD))
                .py(px(style::CHIP_PADDING_Y))
                .max_w(px(460.0))
                .rounded(px(radius::SM))
                .bg(interactive_default)
                .border_1()
                .border_color(border_subtle)
                .text_size(px(style::label(false)))
                .text_color(theme.text.primary)
                .overflow_hidden()
                .when_some(icon, |el, icon| {
                    el.child(
                        div()
                            .text_size(px(style::icon(false)))
                            .text_color(theme.text.secondary)
                            .child(icon),
                    )
                })
                .child(div().overflow_hidden().text_ellipsis().child(title))
        }
    }
}

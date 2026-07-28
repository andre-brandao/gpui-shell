//! Active window widget displaying the title of the currently focused window.

mod config;
pub use config::ActiveWindowConfig;

use gpui::{AnyElement, Context, Render, Window, div, prelude::*, px};
use services::CompositorState;

use super::{BarWidget, BarWidgetShell, style};
use crate::config::ActiveConfig;
use crate::state::AppState;
use crate::state::watch;
use ui::ActiveTheme;

/// Widget that displays the currently focused window's title.
pub struct ActiveWindow {
    _compositor: services::CompositorSubscriber,
    state: CompositorState,
}

impl ActiveWindow {
    const VERTICAL_LINE_WIDTH: usize = 3;
    const VERTICAL_MAX_LINES: usize = 5;

    /// Create a new active window widget.
    pub fn new(cx: &mut Context<Self>) -> Self {
        let compositor = AppState::compositor(cx).clone();
        let state = compositor.get();

        // Subscribe to compositor state changes
        watch(cx, compositor.subscribe(), |this, new_state, cx| {
            this.state = new_state;
            cx.notify();
        });

        Self {
            _compositor: compositor,
            state,
        }
    }

    fn window_text(&self) -> &str {
        let Some(window) = self.state.active_window.as_ref() else {
            return "";
        };

        let title = window.title.trim();
        if !title.is_empty() {
            return title;
        }

        window.class.trim()
    }

    /// Get the display title, truncated if necessary.
    fn display_title(&self, max_length: usize) -> String {
        let title = self.window_text();
        if title.is_empty() {
            return String::new();
        }

        if max_length == 0 {
            return title.to_string();
        }

        if let Some((cutoff, _)) = title.char_indices().nth(max_length) {
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

    fn vertical_lines(&self, max_length: usize) -> Vec<String> {
        let source = self.window_text();
        if source.is_empty() {
            return Vec::new();
        }
        let max_length = if max_length == 0 {
            usize::MAX
        } else {
            max_length
        };
        let max_lines = Self::VERTICAL_MAX_LINES;
        let line_width = Self::VERTICAL_LINE_WIDTH;

        let filtered: String = source
            .chars()
            .filter(|ch| *ch != '\n' && *ch != '\r')
            .take(max_length)
            .collect();
        let was_truncated = source
            .chars()
            .filter(|ch| *ch != '\n' && *ch != '\r')
            .count()
            > filtered.chars().count();

        let tokens: Vec<String> = filtered
            .split(|ch: char| !ch.is_alphanumeric())
            .filter(|segment| !segment.is_empty())
            .map(|segment| Self::vertical_segment(segment, line_width))
            .filter(|segment| !segment.is_empty())
            .collect();

        let mut lines = if tokens.len() > 1 {
            tokens
        } else {
            let condensed: String = filtered.chars().filter(|ch| ch.is_alphanumeric()).collect();
            Self::chunk_vertical_text(&condensed, line_width)
        };

        if lines.is_empty() {
            lines = Self::chunk_vertical_text(&filtered, line_width);
        }

        let clipped = lines.len() > max_lines;
        lines.truncate(max_lines);

        if was_truncated || clipped {
            Self::mark_vertical_overflow(&mut lines);
        }

        lines
    }

    fn vertical_segment(segment: &str, width: usize) -> String {
        segment
            .chars()
            .take(width)
            .collect::<String>()
            .to_uppercase()
    }

    fn chunk_vertical_text(source: &str, width: usize) -> Vec<String> {
        let chars: Vec<char> = source.chars().collect();
        chars
            .chunks(width)
            .map(|chunk| chunk.iter().collect::<String>().to_uppercase())
            .collect()
    }

    fn mark_vertical_overflow(lines: &mut [String]) {
        let Some(last) = lines.last_mut() else {
            return;
        };

        let mut chars: Vec<char> = last.chars().collect();
        if chars.is_empty() {
            *last = "…".to_string();
            return;
        }

        if chars.len() >= 3 {
            chars.truncate(chars.len() - 1);
        } else {
            chars.truncate(1);
        }

        chars.push('…');
        *last = chars.into_iter().collect();
    }

    fn has_window_content(&self) -> bool {
        !self.window_text().is_empty()
    }
}

impl BarWidget for ActiveWindow {
    fn shell(&self) -> BarWidgetShell {
        if self.has_window_content() {
            BarWidgetShell::Standard
        } else {
            BarWidgetShell::None
        }
    }

    fn render_vertical(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        if !self.has_window_content() {
            return div().id("active-window").into_any_element();
        }

        let vertical_lines = self.vertical_lines(15);
        let text_primary = theme.colors.text;
        let text_secondary = theme.colors.text;

        div()
            .id("active-window")
            .flex()
            .flex_col()
            .items_center()
            .gap(px(1.0))
            .children(
                vertical_lines
                    .into_iter()
                    .enumerate()
                    .map(move |(idx, line)| {
                        style::vertical_text_line(
                            div()
                                .text_size(style::label_size(true).rems())
                                .text_color(if idx == 0 {
                                    text_primary
                                } else {
                                    text_secondary
                                })
                                .child(line),
                        )
                    }),
            )
            .into_any_element()
    }

    fn render_horizontal(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        if !self.has_window_content() {
            return div().id("active-window").into_any_element();
        }

        let config = &cx.config().bar.modules.active_window;
        let title = self.display_title(config.max_length);
        let icon = if config.show_app_icon {
            self.window_icon()
        } else {
            None
        };

        div()
            .id("active-window")
            .flex()
            .items_center()
            .justify_center()
            .gap(px(style::CHIP_GAP))
            .max_w(px(460.0))
            .overflow_hidden()
            .when_some(icon, |el, icon| {
                el.child(
                    div()
                        .flex_shrink_0()
                        .text_size(px(style::icon(false)))
                        .text_color(theme.colors.text)
                        .child(icon),
                )
            })
            .child(
                div()
                    .flex_shrink(1.)
                    .overflow_hidden()
                    .text_ellipsis()
                    .text_size(style::label_size(false).rems())
                    .text_color(theme.colors.text)
                    .child(title),
            )
            .into_any_element()
    }
}

impl Render for ActiveWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_bar_widget(window, cx)
    }
}

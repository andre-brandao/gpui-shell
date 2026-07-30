//! Active window widget displaying the title of the currently focused window.

mod config;
pub use config::ActiveWindowConfig;

use gpui::{AnyElement, Context, Render, Window, div, prelude::*, px};
use services::CompositorState;

use super::{BarWidget, BarWidgetShell, style};
use crate::config::ActiveConfig;
use crate::state::AppState;
use crate::state::watch;
use ui::{ActiveTheme, Color, Icon, IconName, LabelCommon, h_flex, v_flex};

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

    /// Substring to icon, in match order.
    ///
    /// Brand marks first, then the "what it *is*" fallbacks - an editor we
    /// don't have a logo for is still a `<>`, a terminal is still a prompt.
    /// Order carries real weight here: the haystack is class *and* title, so
    /// a generic needle like "code" would otherwise swallow any window whose
    /// title merely mentions it.
    const WINDOW_ICON_HINTS: &[(&str, IconName)] = &[
        ("firefox", IconName::Firefox),
        // "chromium" does not contain "chrome", so both spellings are needed.
        ("chrome", IconName::Chrome),
        ("chromium", IconName::Chrome),
        ("telegram", IconName::Telegram),
        ("discord", IconName::Discord),
        ("vesktop", IconName::Discord),
        ("slack", IconName::Slack),
        ("spotify", IconName::Spotify),
        ("thunderbird", IconName::Thunderbird),
        ("alacritty", IconName::Alacritty),
        ("wezterm", IconName::Wezterm),
        ("neovim", IconName::Neovim),
        ("nvim", IconName::Neovim),
        // Zed's app id, not a bare "zed": that is a substring of "optimized"
        // and "analyzed", which any editor could have in a window title.
        ("dev.zed", IconName::Zed),
        ("code", IconName::VisualStudioCode),
        ("kitty", IconName::Terminal),
        ("terminal", IconName::Terminal),
        ("nautilus", IconName::Folder),
        ("thunar", IconName::Folder),
        ("dolphin", IconName::Folder),
        ("files", IconName::Folder),
    ];

    /// Icon for the focused window, keyed off its class/title.
    ///
    /// Anything unrecognised falls back to a generic window.
    fn window_icon(&self) -> Option<IconName> {
        let window = self.state.active_window.as_ref()?;
        let haystack = format!(
            "{} {}",
            window.class.to_lowercase(),
            window.title.to_lowercase()
        );

        Some(
            Self::WINDOW_ICON_HINTS
                .iter()
                .find(|(needle, _)| haystack.contains(needle))
                .map_or(IconName::Layout, |&(_, icon)| icon),
        )
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
        let text_color = theme.colors.text;

        v_flex()
            .id("active-window")
            .items_center()
            .gap(px(1.0))
            .children(vertical_lines.into_iter().map(move |line| {
                style::vertical_text_line(style::bar_label(line, true, text_color))
            }))
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

        h_flex()
            .id("active-window")
            .justify_center()
            .gap(px(style::CHIP_GAP))
            .max_w(px(460.0))
            .overflow_hidden()
            .when_some(icon, |el, icon| {
                el.child(
                    Icon::new(icon)
                        .size(style::icon(false))
                        .color(Color::Default),
                )
            })
            .child(style::bar_label(title, false, theme.colors.text).truncate())
            .into_any_element()
    }
}

impl Render for ActiveWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_bar_widget(window, cx)
    }
}

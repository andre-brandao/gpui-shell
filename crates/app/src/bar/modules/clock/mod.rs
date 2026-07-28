//! Clock widget that displays the current date and time.

mod config;
pub use config::ClockConfig;

use chrono::Local;
use gpui::{AnyElement, Context, Window, div, prelude::*, px};
use std::time::Duration;
use ui::ActiveTheme;

use super::{BarWidget, style};
use crate::config::ActiveConfig;

/// A clock widget that updates every second.
pub struct Clock;

impl Clock {
    /// Create a new clock widget that auto-updates.
    pub fn new(cx: &mut Context<Self>) -> Self {
        // Spawn a timer to update the clock every second.
        cx.spawn(async move |this, cx| {
            loop {
                let _ = this.update(cx, |_, cx| cx.notify());
                cx.background_executor().timer(Duration::from_secs(1)).await;
            }
        })
        .detach();

        Clock
    }

    fn formatted_time_horizontal(&self, format: &str) -> String {
        Local::now().format(format).to_string()
    }

    fn formatted_time_vertical(&self, format: &str) -> Vec<String> {
        Local::now()
            .format(format)
            .to_string()
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect()
    }

    fn render_vertical_content(
        &self,
        theme: &ui::Theme,
        vertical_format: &str,
        horizontal_fallback: &str,
    ) -> AnyElement {
        let mut lines = self.formatted_time_vertical(vertical_format);
        if lines.is_empty() {
            lines.push(self.formatted_time_horizontal(horizontal_fallback));
        }

        div()
            .id("clock")
            .flex()
            .flex_col()
            .items_center()
            .gap(px(style::CHIP_GAP))
            .children(lines.into_iter().enumerate().map(|(idx, line)| {
                style::vertical_text_line(
                    div()
                        .text_size(style::label_size(true).rems())
                        .text_color(if idx == 0 {
                            theme.colors.text_muted
                        } else {
                            theme.colors.text
                        })
                        .child(line),
                )
            }))
            .into_any_element()
    }

    fn render_horizontal_content(&self, theme: &ui::Theme, format: &str) -> AnyElement {
        div()
            .id("clock")
            .flex()
            .items_center()
            .gap(px(style::CHIP_GAP))
            .text_size(style::label_size(false).rems())
            .text_color(theme.colors.text)
            .child(self.formatted_time_horizontal(format))
            .into_any_element()
    }
}

impl BarWidget for Clock {
    fn render_vertical(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let config = &cx.config().bar.modules.clock;

        self.render_vertical_content(theme, &config.format_vertical, &config.format_horizontal)
    }
    fn render_horizontal(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let config = &cx.config().bar.modules.clock;

        self.render_horizontal_content(theme, &config.format_horizontal)
    }
}

impl Render for Clock {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_bar_widget(window, cx)
    }
}

//! Clock widget that displays the current date and time.

mod config;
pub use config::ClockConfig;

use chrono::Local;
use gpui::{Context, Window, div, prelude::*, px};
use std::time::Duration;
use ui::{ActiveTheme, radius};

use super::style;
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
}

impl Render for Clock {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let is_vertical = cx.config().bar.is_vertical();
        let config = &cx.config().bar.modules.clock;

        if is_vertical {
            let mut lines = self.formatted_time_vertical(&config.format_vertical);
            if lines.is_empty() {
                lines.push(self.formatted_time_horizontal(&config.format_horizontal));
            }
            div()
                .id("clock")
                .flex()
                .flex_col()
                .items_center()
                .gap(px(style::CHIP_GAP))
                .px(px(style::chip_padding_x(true)))
                .py(px(style::CHIP_PADDING_Y))
                .rounded(px(radius::SM))
                .children(lines.into_iter().enumerate().map(|(idx, line)| {
                    div()
                        .text_size(style::label_size(theme, true))
                        .text_color(if idx == 0 {
                            theme.text.secondary
                        } else {
                            theme.text.primary
                        })
                        .child(line)
                }))
        } else {
            div()
                .id("clock")
                .flex()
                .items_center()
                .gap(px(style::CHIP_GAP))
                .px(px(style::chip_padding_x(false)))
                .py(px(style::CHIP_PADDING_Y))
                .rounded(px(radius::SM))
                .text_size(style::label_size(theme, false))
                .text_color(theme.text.primary)
                .child(self.formatted_time_horizontal(&config.format_horizontal))
        }
    }
}

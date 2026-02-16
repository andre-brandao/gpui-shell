//! Clock widget that displays the current date and time.

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

    fn formatted_time_horizontal(&self) -> String {
        Local::now().format("%a %H:%M").to_string()
    }

    fn formatted_time_vertical(&self) -> (String, String) {
        let now = Local::now();
        (now.format("%H").to_string(), now.format("%M").to_string())
    }
}

impl Render for Clock {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let is_vertical = cx.config().bar.is_vertical();

        if is_vertical {
            let (hours, minutes) = self.formatted_time_vertical();
            div()
                .id("clock")
                .flex()
                .flex_col()
                .items_center()
                .gap(px(style::CHIP_GAP))
                .px(px(style::chip_padding_x(true)))
                .py(px(style::CHIP_PADDING_Y))
                .rounded(px(radius::SM))
                .child(
                    div()
                        .text_size(px(style::label(true)))
                        .text_color(theme.text.secondary)
                        .child(hours),
                )
                .child(
                    div()
                        .text_size(px(style::label(true)))
                        .text_color(theme.text.primary)
                        .child(minutes),
                )
        } else {
            div()
                .id("clock")
                .flex()
                .items_center()
                .gap(px(style::CHIP_GAP))
                .px(px(style::chip_padding_x(false)))
                .py(px(style::CHIP_PADDING_Y))
                .rounded(px(radius::SM))
                .text_size(px(style::label(false)))
                .text_color(theme.text.primary)
                .child(self.formatted_time_horizontal())
        }
    }
}

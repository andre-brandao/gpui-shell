//! Clock widget that displays the current date and time.

use chrono::Local;
use gpui::{Context, Window, div, prelude::*};
use std::time::Duration;
use ui::{ActiveTheme, font_size};

use crate::config::ActiveConfig;

/// A clock widget that updates every second.
pub struct Clock;

impl Clock {
    /// Create a new clock widget that auto-updates.
    pub fn new(cx: &mut Context<Self>) -> Self {
        // Spawn a timer to update the clock every 500ms
        cx.spawn(async move |this, cx| {
            loop {
                let _ = this.update(cx, |_, cx| cx.notify());
                cx.background_executor()
                    .timer(Duration::from_millis(500))
                    .await;
            }
        })
        .detach();

        Clock
    }

    /// Get the formatted date and time string.
    fn formatted_time(&self) -> String {
        Local::now().format("%H:%M:%S %d/%m/%Y").to_string()
    }

    fn formatted_time_compact(&self) -> String {
        Local::now().format("%H:%M").to_string()
    }

    fn formatted_date_compact(&self) -> String {
        Local::now().format("%d/%m").to_string()
    }
}

impl Render for Clock {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let is_vertical = cx.config().bar.orientation.is_vertical();

        if is_vertical {
            div()
                .flex()
                .flex_col()
                .items_center()
                .text_size(gpui::px(font_size::SM))
                .text_color(theme.text.primary)
                .child(self.formatted_time_compact())
                .child(
                    div()
                        .text_size(gpui::px(font_size::XS))
                        .text_color(theme.text.secondary)
                        .child(self.formatted_date_compact()),
                )
        } else {
            div()
                .flex()
                .items_center()
                .text_size(gpui::px(font_size::BASE))
                .text_color(theme.text.primary)
                .child(self.formatted_time())
        }
    }
}

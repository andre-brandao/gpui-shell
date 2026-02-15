//! Clock widget that displays the current date and time.

use chrono::Local;
use gpui::{Context, Window, div, prelude::*};
use std::time::Duration;
use ui::{ActiveTheme, font_size};

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
}

impl Render for Clock {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .flex()
            .items_center()
            .text_size(gpui::px(font_size::BASE))
            .text_color(theme.text.primary)
            .child(self.formatted_time())
    }
}

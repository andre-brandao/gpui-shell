//! Clock widget using Zed UI `Label`.
//!
//! Keeps a simple timer-based update loop.

use chrono::Local;
use gpui::{Context, Window, prelude::*};
use std::time::Duration;
use ui::{Color, Label, LabelSize, prelude::*};

pub struct Clock {
    time_str: String,
}

impl Clock {
    pub fn new(cx: &mut Context<Self>) -> Self {
        // Update every second.
        cx.spawn(async move |this, cx| {
            loop {
                let _ = this.update(cx, |this, cx| {
                    this.time_str = Self::now_string();
                    cx.notify();
                });
                cx.background_executor().timer(Duration::from_secs(1)).await;
            }
        })
        .detach();

        Self {
            time_str: Self::now_string(),
        }
    }

    fn now_string() -> String {
        Local::now().format("%d/%m/%Y %H:%M:%S").to_string()
    }
}

impl Render for Clock {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        Label::new(self.time_str.clone())
            .size(LabelSize::Small)
            .color(Color::Default)
    }
}

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use gpui::{Context, Window, div, prelude::*};

pub struct Clock;

impl Clock {
    pub fn new(cx: &mut Context<Self>) -> Self {
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

    fn get_time(&self) -> (u64, u64, u64) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let hours = (now / 3600) % 24;
        let minutes = (now / 60) % 60;
        let seconds = now % 60;

        (hours, minutes, seconds)
    }
}

impl Render for Clock {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let (hours, minutes, seconds) = self.get_time();

        div()
            .flex()
            .items_center()
            .child(format!("{:02}:{:02}:{:02}", hours, minutes, seconds))
    }
}

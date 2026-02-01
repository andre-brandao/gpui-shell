use gpui::{div, prelude::*};

pub fn clock(hours: u64, minutes: u64, seconds: u64) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .child(format!("{:02}:{:02}:{:02}", hours, minutes, seconds))
}

use std::time::{SystemTime, UNIX_EPOCH};

use gpui::{div, prelude::*};

pub fn clock() -> impl IntoElement {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let hours = (now / 3600) % 24;
    let minutes = (now / 60) % 60;
    let seconds = now % 60;

    div()
        .flex()
        .items_center()
        .child(format!("{:02}:{:02}:{:02}", hours, minutes, seconds))
}

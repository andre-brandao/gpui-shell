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

    fn get_time(&self) -> (u64, u64, u64, u64, u64, u64) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let hours = (now / 3600) % 24;
        let minutes = (now / 60) % 60;
        let seconds = now % 60;

        // Calculate date
        let days_since_epoch = now / 86400;
        let (year, month, day) = days_to_ymd(days_since_epoch);

        (hours, minutes, seconds, day, month, year)
    }
}

fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    // Days since 1970-01-01
    let mut remaining = days as i64;
    let mut year = 1970i64;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        year += 1;
    }

    let leap = is_leap_year(year);
    let days_in_months: [i64; 12] = if leap {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 0;
    for (i, &days_in_month) in days_in_months.iter().enumerate() {
        if remaining < days_in_month {
            month = i + 1;
            break;
        }
        remaining -= days_in_month;
    }

    let day = remaining + 1;
    (year as u64, month as u64, day as u64)
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

impl Render for Clock {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let (hours, minutes, seconds, day, month, year) = self.get_time();

        div().flex().items_center().child(format!(
            "{:02}/{:02}/{} {:02}:{:02}:{:02}",
            day, month, year, hours, minutes, seconds
        ))
    }
}

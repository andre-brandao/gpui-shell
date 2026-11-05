//! Clock widget that displays the current date and time.

mod config;
pub use config::ClockConfig;

use chrono::{DateTime, Local};
use gpui::{AnyElement, Context, Window, prelude::*};
use std::fmt::Write as _;
use std::time::Duration;
use ui::IconName;

use super::{BarCtx, BarWidget, BarWidgetDescriptor, BarWidgetMeta, boxed, style};
use crate::config::ActiveConfig;

pub const fn descriptor() -> BarWidgetDescriptor {
    BarWidgetDescriptor::new(
        BarWidgetMeta {
            name: "Clock",
            aliases: &[],
            label: "Clock",
            description: "Current time and date.",
            icon: IconName::Clock,
        },
        |cx| boxed(cx, Clock::new),
    )
}

/// Shown when the configured format is not valid strftime.
const FALLBACK_FORMAT: &str = "%H:%M";

/// Time left in the current wall-clock second, so ticks do not drift.
fn until_next_second() -> Duration {
    let nanos = u64::from(Local::now().timestamp_subsec_nanos()) % 1_000_000_000;
    Duration::from_nanos(1_000_000_000 - nanos)
}

/// chrono's `to_string()` panics on a bad specifier; `write!` returns `Err`.
fn format_at(now: DateTime<Local>, format: &str) -> String {
    let mut out = String::new();
    if write!(out, "{}", now.format(format)).is_err() {
        out.clear();
        let _ = write!(out, "{}", now.format(FALLBACK_FORMAT));
    }
    out
}

/// A clock widget that redraws when its text changes.
pub struct Clock;

impl Clock {
    /// Create a new clock widget that auto-updates.
    pub fn new(cx: &mut Context<Self>) -> Self {
        cx.spawn(async move |this, cx| {
            let mut shown = Vec::new();
            loop {
                cx.background_executor().timer(until_next_second()).await;
                // Notify only on new text: a format without seconds redraws once a minute.
                let alive = this.update(cx, |_, cx| {
                    let bar = &cx.config().bar;
                    let clock = &bar.modules.clock;
                    let text = if bar.is_vertical() {
                        Self::lines(&clock.format_vertical, &clock.format_horizontal)
                    } else {
                        vec![Self::formatted(&clock.format_horizontal)]
                    };
                    if text != shown {
                        shown = text;
                        cx.notify();
                    }
                });
                if alive.is_err() {
                    break;
                }
            }
        })
        .detach();

        Clock
    }

    fn formatted(format: &str) -> String {
        format_at(Local::now(), format)
    }

    /// One entry per non-empty line of the vertical format, falling back to
    /// the horizontal format when it yields nothing.
    fn lines(vertical_format: &str, horizontal_fallback: &str) -> Vec<String> {
        let lines: Vec<String> = Self::formatted(vertical_format)
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect();
        if lines.is_empty() {
            vec![Self::formatted(horizontal_fallback)]
        } else {
            lines
        }
    }
}

impl BarWidget for Clock {
    fn render_vertical(
        &mut self,
        ctx: &BarCtx,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let config = &cx.config().bar.modules.clock;
        let lines = Self::lines(&config.format_vertical, &config.format_horizontal);

        style::stack(ctx)
            .id("clock")
            .gap(ctx.metrics.gap)
            .children(lines.into_iter().enumerate().map(|(idx, line)| {
                // Hours and minutes carry the time; anything after (seconds,
                // a date) is secondary and muted.
                let color = if idx < 2 { ctx.ink.text } else { ctx.ink.muted };
                style::text_line(ctx, style::label_in(ctx, line, color))
            }))
            .into_any_element()
    }

    fn render_horizontal(
        &mut self,
        ctx: &BarCtx,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let config = &cx.config().bar.modules.clock;

        style::stack(ctx)
            .id("clock")
            .gap(ctx.metrics.gap)
            .child(style::label(
                ctx,
                Self::formatted(&config.format_horizontal),
            ))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn bad_format_falls_back_instead_of_panicking() {
        let t = Local.with_ymd_and_hms(2024, 1, 2, 3, 4, 5).unwrap();
        assert_eq!(format_at(t, "%Q"), "03:04");
        assert_eq!(format_at(t, "%H:%M"), "03:04");
        assert_eq!(format_at(t, "%S"), "05");
        assert_eq!(Clock::lines("%Q", "%H").len(), 1);
    }

    #[test]
    fn tick_lands_inside_the_next_second() {
        let delay = until_next_second();
        assert!(delay > Duration::ZERO && delay <= Duration::from_secs(1));
    }
}

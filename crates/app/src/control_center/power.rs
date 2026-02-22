//! Power section for the Control Center.
//!
//! Displays power actions when expanded.

use crate::config::ActiveConfig;
use gpui::{App, Hsla, MouseButton, div, prelude::*, px};
use ui::{ActiveTheme, icon_size, radius, spacing};

use super::{config::PowerActionsConfig, icons};

/// Render the power section (expanded view with actions)
pub fn render_power_section(cx: &App) -> impl IntoElement {
    div().w_full().flex().flex_col().child(render_power_actions(
        &cx.config().control_center.power_actions,
        cx,
    ))
}

/// Render power action buttons
fn render_power_actions(config: &PowerActionsConfig, cx: &App) -> impl IntoElement {
    div()
        .flex()
        .gap(px(spacing::XS))
        .items_center()
        .child(render_action_button(
            "power-action-sleep",
            icons::POWER_SLEEP,
            "Sleep",
            &config.sleep,
            cx,
        ))
        .child(render_action_button(
            "power-action-reboot",
            icons::REFRESH,
            "Reboot",
            &config.reboot,
            cx,
        ))
        .child(render_action_button(
            "power-action-poweroff",
            icons::POWER_BUTTON,
            "Power off",
            &config.poweroff,
            cx,
        ))
}

/// Render a power action button
fn render_action_button(
    id: &'static str,
    icon: &'static str,
    label: &'static str,
    command: &str,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();

    let interactive_default = theme.interactive.default;
    let interactive_hover = theme.interactive.hover;
    let text_primary = theme.text.primary;
    let text_muted = theme.text.muted;

    let command = command.trim().to_string();
    let enabled = !command.is_empty();
    let fg_color: Hsla = if enabled { text_primary } else { text_muted };

    div()
        .id(id)
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .gap(px(spacing::XS))
        .py(px(spacing::SM))
        .rounded(px(radius::SM))
        .bg(interactive_default)
        .when(enabled, move |el| {
            let command = command.clone();
            el.cursor_pointer()
                .hover(move |s| s.bg(interactive_hover))
                .on_mouse_down(MouseButton::Left, move |_, _, _| {
                    run_power_command(command.clone());
                })
        })
        .child(
            div()
                .text_size(px(icon_size::SM))
                .text_color(fg_color)
                .child(icon),
        )
        .child(
            div()
                .text_size(theme.font_sizes.xs)
                .text_color(fg_color)
                .child(label),
        )
}

fn run_power_command(command: String) {
    if command.trim().is_empty() {
        return;
    }

    std::thread::spawn(move || {
        if let Err(err) = std::process::Command::new("sh")
            .args(["-c", &command])
            .spawn()
        {
            tracing::warn!("Failed to run power action '{}': {}", command, err);
        }
    });
}

/// Format time remaining for battery
pub(crate) fn format_time_remaining(battery: &services::BatteryData) -> Option<String> {
    let duration = if battery.is_charging() {
        battery.time_to_full.as_ref()
    } else {
        battery.time_to_empty.as_ref()
    };

    let seconds = duration?.as_secs();
    if seconds == 0 {
        return None;
    }

    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;

    Some(if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else {
        format!("{}m", minutes)
    })
}

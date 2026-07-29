//! Keyboard layout widget for displaying and cycling keyboard layouts.

mod config;
pub use config::KeyboardLayoutConfig;

use gpui::{AnyElement, Context, MouseButton, Window, div, prelude::*, px};
use services::{CompositorCommand, CompositorState};
use ui::{ActiveTheme, Color, Icon, IconName};

use super::{BarWidget, style};
use crate::config::ActiveConfig;
use crate::state::AppState;
use crate::state::watch;

const KEYBOARD_ICON: IconName = IconName::Keyboard;

/// Keyboard layout widget that displays the current keyboard layout.
pub struct KeyboardLayout {
    compositor: services::CompositorSubscriber,
    state: CompositorState,
}

impl KeyboardLayout {
    /// Create a new KeyboardLayout widget.
    pub fn new(cx: &mut Context<Self>) -> Self {
        let compositor = AppState::compositor(cx).clone();
        let state = compositor.get();

        // Subscribe to compositor state changes
        watch(
            cx,
            AppState::compositor(cx).subscribe(),
            |this, new_state, cx| {
                this.state = new_state;
                cx.notify();
            },
        );

        Self { compositor, state }
    }

    /// Cycle to the next keyboard layout.
    fn next_layout(&self) {
        if let Err(e) = self
            .compositor
            .dispatch(CompositorCommand::NextKeyboardLayout)
        {
            tracing::error!("Failed to switch keyboard layout: {}", e);
        }
    }

    /// Get a short display name for the keyboard layout.
    /// Converts full layout names like "English (US)" to short codes like "EN".
    fn short_layout_name(&self) -> String {
        let layout = &self.state.keyboard_layout;
        let layout_lower = layout.to_lowercase();

        // Common layout name mappings
        let short = if layout_lower.contains("english") {
            "EN"
        } else if layout_lower.contains("russian") {
            "RU"
        } else if layout_lower.contains("german") {
            "DE"
        } else if layout_lower.contains("french") {
            "FR"
        } else if layout_lower.contains("spanish") {
            "ES"
        } else if layout_lower.contains("italian") {
            "IT"
        } else if layout_lower.contains("portuguese") {
            "PT"
        } else if layout_lower.contains("japanese") {
            "JP"
        } else if layout_lower.contains("chinese") {
            "CN"
        } else if layout_lower.contains("korean") {
            "KR"
        } else if layout_lower.contains("arabic") {
            "AR"
        } else if layout_lower.contains("hebrew") {
            "HE"
        } else if layout_lower.contains("ukrainian") {
            "UA"
        } else if layout_lower.contains("polish") {
            "PL"
        } else if layout_lower.contains("czech") {
            "CZ"
        } else if layout_lower.contains("dutch") {
            "NL"
        } else if layout_lower.contains("swedish") {
            "SE"
        } else if layout_lower.contains("norwegian") {
            "NO"
        } else if layout_lower.contains("danish") {
            "DK"
        } else if layout_lower.contains("finnish") {
            "FI"
        } else if layout_lower.contains("turkish") {
            "TR"
        } else if layout_lower.contains("greek") {
            "GR"
        } else if layout.chars().count() >= 2 {
            // Fallback: take first 2 Unicode chars uppercase.
            return layout.chars().take(2).collect::<String>().to_uppercase();
        } else {
            return layout.to_uppercase();
        };

        short.to_uppercase()
    }

    fn flag_for_layout(&self) -> Option<&'static str> {
        match self.short_layout_name().as_str() {
            "EN" => Some("🇺🇸"),
            "RU" => Some("🇷🇺"),
            "DE" => Some("🇩🇪"),
            "FR" => Some("🇫🇷"),
            "ES" => Some("🇪🇸"),
            "IT" => Some("🇮🇹"),
            "PT" => Some("🇵🇹"),
            "JP" => Some("🇯🇵"),
            "CN" => Some("🇨🇳"),
            "KR" => Some("🇰🇷"),
            "AR" => Some("🇸🇦"),
            "HE" => Some("🇮🇱"),
            "UA" => Some("🇺🇦"),
            "PL" => Some("🇵🇱"),
            "CZ" => Some("🇨🇿"),
            "NL" => Some("🇳🇱"),
            "SE" => Some("🇸🇪"),
            "NO" => Some("🇳🇴"),
            "DK" => Some("🇩🇰"),
            "FI" => Some("🇫🇮"),
            "TR" => Some("🇹🇷"),
            "GR" => Some("🇬🇷"),
            _ => None,
        }
    }

    /// `flag` is a country emoji when the user asked for one and we know it;
    /// otherwise the widget falls back to the keyboard icon.
    fn render_layout_content(
        &self,
        theme: &ui::Theme,
        flag: Option<&'static str>,
        short_name: String,
        is_vertical: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .id("keyboard-layout")
            .flex()
            .when(is_vertical, |el| el.flex_col())
            .items_center()
            .justify_center()
            .gap(px(style::CHIP_GAP))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _event, _window, _cx| {
                    this.next_layout();
                }),
            )
            .map(|el| match flag {
                Some(flag) => el.child(
                    div()
                        .flex_shrink_0()
                        .text_size(style::icon(is_vertical).rems())
                        .child(flag),
                ),
                None => el.child(
                    Icon::new(KEYBOARD_ICON)
                        .size(style::icon(is_vertical))
                        .color(Color::Default),
                ),
            })
            .child(if is_vertical {
                style::vertical_text_line(
                    div()
                        .flex_shrink_0()
                        .text_size(style::label_size(is_vertical).rems())
                        .text_color(theme.colors.text)
                        .child(short_name),
                )
            } else {
                div()
                    .flex_shrink_0()
                    .text_size(style::label_size(is_vertical).rems())
                    .text_color(theme.colors.text)
                    .child(short_name)
                    .into_any_element()
            })
            .into_any_element()
    }
}

impl BarWidget for KeyboardLayout {
    fn is_interactive(&self) -> bool {
        true
    }

    fn render_vertical(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        let config = &cx.config().bar.modules.keyboard_layout;
        let short_name = self.short_layout_name();
        let flag = config.show_flag.then(|| self.flag_for_layout()).flatten();
        self.render_layout_content(&theme, flag, short_name, true, cx)
    }

    fn render_horizontal(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        let config = &cx.config().bar.modules.keyboard_layout;
        let short_name = self.short_layout_name();
        let flag = config.show_flag.then(|| self.flag_for_layout()).flatten();
        self.render_layout_content(&theme, flag, short_name, false, cx)
    }
}

impl Render for KeyboardLayout {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_bar_widget(window, cx)
    }
}

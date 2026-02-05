//! Keyboard layout widget for displaying and cycling layouts.

use futures_signals::signal::SignalExt;
use futures_util::StreamExt;
use gpui::{Context, MouseButton, Window, div, prelude::*, px, rems};
use services::{CompositorCommand, CompositorState, CompositorSubscriber, Services};
use ui::prelude::*;

pub struct KeyboardLayout {
    compositor: CompositorSubscriber,
    state: CompositorState,
}

impl KeyboardLayout {
    pub fn new(services: Services, cx: &mut Context<Self>) -> Self {
        let compositor = services.compositor.clone();
        let state = compositor.get();

        // Subscribe to compositor state changes
        cx.spawn({
            let mut signal = compositor.subscribe().to_stream();
            async move |this, cx| {
                while let Some(new_state) = signal.next().await {
                    if this
                        .update(cx, |this, cx| {
                            this.state = new_state;
                            cx.notify();
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            }
        })
        .detach();

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
    fn short_layout_name(&self) -> String {
        let layout = &self.state.keyboard_layout;
        let lower = layout.to_lowercase();

        let short = if lower.contains("english") {
            "EN"
        } else if lower.contains("russian") {
            "RU"
        } else if lower.contains("german") {
            "DE"
        } else if lower.contains("french") {
            "FR"
        } else if lower.contains("spanish") {
            "ES"
        } else if lower.contains("italian") {
            "IT"
        } else if lower.contains("portuguese") {
            "PT"
        } else if lower.contains("japanese") {
            "JP"
        } else if lower.contains("chinese") {
            "CN"
        } else if lower.contains("korean") {
            "KR"
        } else if lower.contains("arabic") {
            "AR"
        } else if lower.contains("hebrew") {
            "HE"
        } else if lower.contains("ukrainian") {
            "UA"
        } else if lower.contains("polish") {
            "PL"
        } else if lower.contains("czech") {
            "CZ"
        } else if lower.contains("dutch") {
            "NL"
        } else if lower.contains("swedish") {
            "SE"
        } else if lower.contains("norwegian") {
            "NO"
        } else if lower.contains("danish") {
            "DK"
        } else if lower.contains("finnish") {
            "FI"
        } else if lower.contains("turkish") {
            "TR"
        } else if lower.contains("greek") {
            "GR"
        } else if layout.len() >= 2 {
            &layout[..2]
        } else {
            layout
        };

        short.to_uppercase()
    }
}

impl Render for KeyboardLayout {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors();
        let short_name = self.short_layout_name();

        let hover_bg = colors.element_hover;
        let active_bg = colors.element_active;
        let base_bg = colors.element_background;

        div()
            .id("keyboard-layout")
            .flex()
            .items_center()
            .gap(px(4.0))
            .px(px(8.0))
            .py(px(2.0))
            .rounded(px(6.0))
            .cursor_pointer()
            .bg(base_bg)
            .hover(move |s| s.bg(hover_bg))
            .active(move |s| s.bg(active_bg))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _event, _window, _cx| {
                    this.next_layout();
                }),
            )
            .child(div().text_color(colors.text_muted).child("󰌌 "))
            .child(
                div()
                    .text_color(colors.text)
                    .text_size(rems(0.75))
                    .child(short_name),
            )
    }
}

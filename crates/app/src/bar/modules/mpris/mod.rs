//! MPRIS widget showing active media state and opening a players panel.

mod config;
pub use config::MprisConfig;

use gpui::{AnyElement, App, Context, MouseButton, Window, div, prelude::*, px};
use services::{MprisData, PlaybackStatus};
use ui::ActiveTheme;

use super::{BarWidget, style};
use crate::config::ActiveConfig;
use crate::panel::toggle_widget_panel;
use crate::state::AppState;
use crate::state::watch;

mod panel;
pub use panel::MprisPanel;

mod icons {
    pub const PLAYING: &str = "󰐊";
    pub const PAUSED: &str = "󰏤";
    pub const STOPPED: &str = "󰓛";
}

/// Bar widget for media status and controls.
pub struct Mpris {
    subscriber: services::MprisSubscriber,
    data: MprisData,
}

impl Mpris {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let subscriber = AppState::mpris(cx).clone();
        let data = subscriber.get();

        watch(cx, subscriber.subscribe(), |this, data, cx| {
            this.data = data;
            cx.notify();
        });

        Self { subscriber, data }
    }

    fn toggle_panel(&self, event: &gpui::MouseDownEvent, window: &Window, cx: &mut App) {
        let subscriber = self.subscriber.clone();
        toggle_widget_panel(
            "mpris",
            gpui::Size::new(380.0, 420.0),
            "mpris-panel",
            event,
            window,
            cx,
            move |cx| MprisPanel::new(subscriber, cx),
        );
    }

    fn primary_player(&self) -> Option<&services::MprisPlayerData> {
        self.data
            .players
            .iter()
            .find(|p| p.state == PlaybackStatus::Playing)
            .or_else(|| self.data.players.first())
    }

    fn icon(&self) -> &'static str {
        match self.primary_player().map(|p| p.state) {
            Some(PlaybackStatus::Playing) => icons::PLAYING,
            Some(PlaybackStatus::Paused) => icons::PAUSED,
            Some(PlaybackStatus::Stopped) => icons::STOPPED,
            None => icons::STOPPED,
        }
    }

    fn label(&self) -> String {
        let Some(player) = self.primary_player() else {
            return "No media".to_string();
        };

        if let Some(metadata) = &player.metadata {
            let value = metadata.to_string();
            if !value.is_empty() {
                return value;
            }
        }

        player
            .service
            .rsplit('.')
            .next()
            .unwrap_or("Player")
            .to_string()
    }

    fn render_widget_content(
        &self,
        theme: &ui::Theme,
        icon: &'static str,
        label: Option<String>,
        is_vertical: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let max_width = cx.config().bar.modules.mpris.max_width;

        div()
            .id("mpris-widget")
            .flex()
            .when(is_vertical, |el| el.flex_col())
            .items_center()
            .justify_center()
            .gap(px(style::CHIP_GAP))
            .max_w(px(max_width))
            .overflow_hidden()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event, window, cx| {
                    this.toggle_panel(event, window, cx);
                }),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .text_size(px(style::icon(is_vertical)))
                    .text_color(theme.text.primary)
                    .child(icon),
            )
            .when_some(label, |el, label| {
                el.child(
                    div()
                        .flex_shrink(1.)
                        .overflow_hidden()
                        .text_ellipsis()
                        .text_size(style::label_size(theme, is_vertical))
                        .text_color(theme.text.secondary)
                        .child(label),
                )
            })
            .into_any_element()
    }
}

impl BarWidget for Mpris {
    fn is_interactive(&self) -> bool {
        true
    }

    fn render_vertical(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        self.render_widget_content(&theme, self.icon(), None, true, cx)
    }

    fn render_horizontal(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        self.render_widget_content(&theme, self.icon(), Some(self.label()), false, cx)
    }
}

impl Render for Mpris {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_bar_widget(window, cx)
    }
}

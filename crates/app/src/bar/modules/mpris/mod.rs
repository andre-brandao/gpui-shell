//! MPRIS widget showing active media state and opening a players panel.

mod config;
pub use config::MprisConfig;

use gpui::{AnyElement, App, Context, MouseButton, Size, Window, div, prelude::*, px};
use services::{MprisData, PlaybackStatus};
use ui::{ActiveTheme, Color, Icon, IconName};

use super::{BarWidget, style};
use crate::config::{ActiveConfig, Config};
use crate::panel::{PanelConfig, panel_placement_from_event, toggle_panel};
use crate::state::AppState;
use crate::state::watch;

mod panel;
pub use panel::MprisPanel;

mod icons {
    use ui::IconName;

    pub const PLAYING: IconName = IconName::Play;
    pub const PAUSED: IconName = IconName::Pause;
    pub const STOPPED: IconName = IconName::Stop;
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
        let config = Config::global(cx);
        let panel_size = Size::new(px(380.0), px(420.0));
        let (anchor, margin) =
            panel_placement_from_event(config.bar.position, event, window, cx, panel_size);
        let config = PanelConfig {
            width: 380.0,
            height: 420.0,
            anchor,
            margin,
            namespace: "mpris-panel".to_string(),
        };

        toggle_panel("mpris", config, cx, move |cx| {
            MprisPanel::new(subscriber, cx)
        });
    }

    fn primary_player(&self) -> Option<&services::MprisPlayerData> {
        self.data
            .players
            .iter()
            .find(|p| p.state == PlaybackStatus::Playing)
            .or_else(|| self.data.players.first())
    }

    fn icon(&self) -> IconName {
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
        icon: IconName,
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
                Icon::new(icon)
                    .size(style::icon(is_vertical))
                    .color(Color::Custom(theme.colors.text)),
            )
            .when_some(label, |el, label| {
                el.child(
                    div()
                        .flex_shrink(1.)
                        .overflow_hidden()
                        .text_ellipsis()
                        .text_size(style::label_size(is_vertical).rems())
                        .text_color(theme.colors.text)
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

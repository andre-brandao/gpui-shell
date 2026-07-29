//! MPRIS panel with a list of players and transport controls.

use gpui::{App, Context, FontWeight, MouseButton, Window, div, img, prelude::*, px};
use services::{MprisCommand, MprisData, MprisSubscriber, PlaybackStatus, PlayerCommand};
use ui::patterns::PanelSurface;
use ui::{ActiveTheme, Color, Icon, IconName, IconSize, Radius, Spacing, TextSize};

use crate::config::ActiveConfig;
use crate::state::watch;
/// Panel content for controlling media players exposed via MPRIS.
pub struct MprisPanel {
    subscriber: MprisSubscriber,
    data: MprisData,
}

impl MprisPanel {
    pub fn new(subscriber: MprisSubscriber, cx: &mut Context<Self>) -> Self {
        let data = subscriber.get();

        watch(cx, subscriber.subscribe(), |this, data, cx| {
            this.data = data;
            cx.notify();
        });

        Self { subscriber, data }
    }

    fn run_command(
        cx: &mut App,
        subscriber: MprisSubscriber,
        service_name: String,
        command: PlayerCommand,
    ) {
        cx.spawn(async move |_| {
            let _ = subscriber
                .dispatch(MprisCommand {
                    service_name,
                    command,
                })
                .await;
        })
        .detach();
    }

    fn status_text(state: PlaybackStatus) -> &'static str {
        match state {
            PlaybackStatus::Playing => "Playing",
            PlaybackStatus::Paused => "Paused",
            PlaybackStatus::Stopped => "Stopped",
        }
    }

    fn title_for(player: &services::MprisPlayerData) -> String {
        player
            .metadata
            .as_ref()
            .and_then(|m| m.title.clone())
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| {
                player
                    .service
                    .rsplit('.')
                    .next()
                    .unwrap_or("Unknown Player")
                    .to_string()
            })
    }

    fn subtitle_for(player: &services::MprisPlayerData) -> String {
        let artist = player
            .metadata
            .as_ref()
            .and_then(|m| m.artists.clone())
            .map(|a| a.join(", "))
            .unwrap_or_default();

        let status = Self::status_text(player.state);
        if artist.is_empty() {
            status.to_string()
        } else {
            format!("{artist} - {status}")
        }
    }

    fn sorted_players(&self) -> Vec<services::MprisPlayerData> {
        let mut players = self.data.players.clone();
        players.sort_by_key(|p| match p.state {
            PlaybackStatus::Playing => 0u8,
            PlaybackStatus::Paused => 1u8,
            PlaybackStatus::Stopped => 2u8,
        });
        players
    }

    fn format_duration(us: Option<i64>) -> String {
        let total_secs = us.unwrap_or(0).max(0) / 1_000_000;
        let minutes = total_secs / 60;
        let seconds = total_secs % 60;
        format!("{minutes}:{seconds:02}")
    }

    fn render_control_button(
        id: impl Into<gpui::ElementId>,
        icon: IconName,
        cx: &App,
        on_click: impl Fn(&mut App) + 'static,
    ) -> impl IntoElement {
        let theme = cx.theme();
        let interactive_default = theme.colors.element_background;
        let interactive_hover = theme.colors.element_hover;

        div()
            .id(id.into())
            .w(px(28.))
            .h(px(24.))
            .rounded(Radius::Small.pixels())
            .cursor_pointer()
            .flex()
            .items_center()
            .justify_center()
            .bg(interactive_default)
            .hover(move |el| el.bg(interactive_hover))
            .on_mouse_down(MouseButton::Left, move |_, _, cx| on_click(cx))
            .child(Icon::new(icon).size(IconSize::XSmall).color(Color::Default))
    }

    fn render_player_card(
        &self,
        player: services::MprisPlayerData,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme();
        let is_playing = player.state == PlaybackStatus::Playing;
        let title = Self::title_for(&player);
        let subtitle = Self::subtitle_for(&player);
        let play_icon = if is_playing {
            IconName::Pause
        } else {
            IconName::Play
        };
        let service_name = player.service.clone();
        let subscriber = self.subscriber.clone();
        let show_cover = cx.config().bar.modules.mpris.show_cover;

        let status_color = match player.state {
            PlaybackStatus::Playing => theme.colors.status.success,
            PlaybackStatus::Paused => theme.colors.status.warning,
            PlaybackStatus::Stopped => theme.colors.text_muted,
        };

        let volume = player
            .volume
            .map(|v| format!("{:.0}%", v.clamp(0.0, 100.0)))
            .unwrap_or_else(|| "--".to_string());
        let duration = Self::format_duration(player.duration_us);

        let can_control = player.can_control;
        let service_short = service_name
            .rsplit('.')
            .next()
            .unwrap_or("player")
            .to_string();

        let prev_service = service_name.clone();
        let pp_service = service_name.clone();
        let next_service = service_name.clone();
        let dec_service = service_name.clone();
        let inc_service = service_name.clone();

        let prev_sub = subscriber.clone();
        let pp_sub = subscriber.clone();
        let next_sub = subscriber.clone();
        let dec_sub = subscriber.clone();
        let inc_sub = subscriber;

        div()
            .w_full()
            .p(Spacing::Medium.pixels())
            .bg(theme.colors.surface_background)
            .rounded(Radius::Medium.pixels())
            .border_1()
            .border_color(theme.colors.border_variant)
            .flex()
            .flex_col()
            .gap(Spacing::Medium.pixels())
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(Spacing::Medium.pixels())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(Spacing::Medium.pixels())
                            .child(if show_cover {
                                player
                                    .art_url
                                    .clone()
                                    .map(|source| {
                                        div()
                                            .size(px(34.0))
                                            .rounded(Radius::Small.pixels())
                                            .overflow_hidden()
                                            .border_1()
                                            .border_color(theme.colors.border_variant)
                                            .child(img(source).size_full())
                                            .into_any_element()
                                    })
                                    .unwrap_or_else(|| {
                                        div()
                                            .size(px(34.0))
                                            .rounded(Radius::Small.pixels())
                                            .bg(theme.colors.elevated_surface_background)
                                            .border_1()
                                            .border_color(theme.colors.border_variant)
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(
                                                Icon::new(IconName::Music)
                                                    .size(IconSize::XSmall)
                                                    .color(Color::Default),
                                            )
                                            .into_any_element()
                                    })
                            } else {
                                div()
                                    .size(px(34.0))
                                    .rounded(Radius::Small.pixels())
                                    .bg(theme.colors.elevated_surface_background)
                                    .border_1()
                                    .border_color(theme.colors.border_variant)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        Icon::new(IconName::Music)
                                            .size(IconSize::XSmall)
                                            .color(Color::Default),
                                    )
                                    .into_any_element()
                            })
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .overflow_hidden()
                                    .child(
                                        div()
                                            .text_size(TextSize::Small.rems())
                                            .text_color(theme.colors.text)
                                            .font_weight(FontWeight::MEDIUM)
                                            .overflow_hidden()
                                            .text_ellipsis()
                                            .child(title),
                                    )
                                    .child(
                                        div()
                                            .text_size(TextSize::XSmall.rems())
                                            .text_color(theme.colors.text)
                                            .overflow_hidden()
                                            .text_ellipsis()
                                            .child(subtitle),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .text_size(TextSize::XSmall.rems())
                            .text_color(status_color)
                            .child(format!("{}  {}", Self::status_text(player.state), volume)),
                    ),
            )
            .when(player.duration_us.is_some(), |el| {
                el.child(
                    div()
                        .w_full()
                        .flex()
                        .items_center()
                        .gap(Spacing::XSmall.pixels())
                        .child(
                            Icon::new(IconName::Clock)
                                .size(IconSize::XSmall)
                                .color(Color::Muted),
                        )
                        .child(
                            div()
                                .text_size(TextSize::XSmall.rems())
                                .text_color(theme.colors.text_muted)
                                .child(duration),
                        ),
                )
            })
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(Spacing::XSmall.pixels())
                    .child(
                        div()
                            .text_size(TextSize::XSmall.rems())
                            .text_color(theme.colors.text_muted)
                            .child(service_short),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(Spacing::XSmall.pixels())
                            .when(can_control, |el| {
                                el.child(Self::render_control_button(
                                    format!("mpris-prev-{}", service_name),
                                    IconName::SkipBack,
                                    cx,
                                    move |cx| {
                                        Self::run_command(
                                            cx,
                                            prev_sub.clone(),
                                            prev_service.clone(),
                                            PlayerCommand::Prev,
                                        );
                                    },
                                ))
                                .child(Self::render_control_button(
                                    format!("mpris-play-{}", pp_service),
                                    play_icon,
                                    cx,
                                    move |cx| {
                                        Self::run_command(
                                            cx,
                                            pp_sub.clone(),
                                            pp_service.clone(),
                                            PlayerCommand::PlayPause,
                                        );
                                    },
                                ))
                                .child(
                                    Self::render_control_button(
                                        format!("mpris-next-{}", next_service),
                                        IconName::SkipForward,
                                        cx,
                                        move |cx| {
                                            Self::run_command(
                                                cx,
                                                next_sub.clone(),
                                                next_service.clone(),
                                                PlayerCommand::Next,
                                            );
                                        },
                                    ),
                                )
                            })
                            .when(can_control && player.volume.is_some(), |el| {
                                el.child(Self::render_control_button(
                                    format!("mpris-dec-{}", dec_service),
                                    IconName::Dash,
                                    cx,
                                    move |cx| {
                                        let value = player.volume.unwrap_or(0.0) - 5.0;
                                        Self::run_command(
                                            cx,
                                            dec_sub.clone(),
                                            dec_service.clone(),
                                            PlayerCommand::Volume(value),
                                        );
                                    },
                                ))
                                .child(
                                    Self::render_control_button(
                                        format!("mpris-inc-{}", inc_service),
                                        IconName::Plus,
                                        cx,
                                        move |cx| {
                                            let value = player.volume.unwrap_or(0.0) + 5.0;
                                            Self::run_command(
                                                cx,
                                                inc_sub.clone(),
                                                inc_service.clone(),
                                                PlayerCommand::Volume(value),
                                            );
                                        },
                                    ),
                                )
                            }),
                    ),
            )
    }
}

impl Render for MprisPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let players = self.sorted_players();
        let is_empty = players.is_empty();

        div()
            .id("mpris-panel")
            .w_full()
            .h_full()
            .p(Spacing::XLarge.pixels())
            .panel_surface(cx)
            .overflow_hidden()
            .child(
                div()
                    .w_full()
                    .h_full()
                    .flex()
                    .flex_col()
                    .gap(Spacing::Large.pixels())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(Spacing::Medium.pixels())
                            .child(
                                Icon::new(IconName::Volume)
                                    .size(IconSize::Large)
                                    .color(Color::Default),
                            )
                            .child(
                                div()
                                    .text_size(TextSize::Large.rems())
                                    .text_color(theme.colors.text)
                                    .font_weight(FontWeight::BOLD)
                                    .child("Media Players"),
                            ),
                    )
                    .when(is_empty, |el| {
                        el.child(
                            div()
                                .flex_1()
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(TextSize::Small.rems())
                                .text_color(theme.colors.text_muted)
                                .child("No MPRIS players detected"),
                        )
                    })
                    .when(!is_empty, |el| {
                        el.child(
                            div()
                                .flex_1()
                                .overflow_hidden()
                                .flex()
                                .flex_col()
                                .gap(Spacing::Medium.pixels())
                                .children(players.into_iter().map(|player| {
                                    self.render_player_card(player, cx).into_any_element()
                                })),
                        )
                    }),
            )
    }
}

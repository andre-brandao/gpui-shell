//! MPRIS panel with a list of players and transport controls.

use gpui::{App, Context, FontWeight, Window, div, img, prelude::*, px};
use services::{MprisCommand, MprisData, MprisSubscriber, PlaybackStatus, PlayerCommand};
use ui::patterns::PanelSurface;
use ui::{
    ActiveTheme, ButtonCommon, ButtonSize, ButtonStyle, Clickable, Color, Icon, IconButton,
    IconName, IconSize, Label, LabelCommon, Radius, Spacing, TextSize, h_flex, v_flex,
};

use crate::config::ActiveConfig;
use crate::state::watch;
/// Panel content for controlling media players exposed via MPRIS.
pub struct MprisPanel {
    subscriber: MprisSubscriber,
    data: MprisData,
}

/// Side of the square album-art slot.
const COVER_SIZE: f32 = 34.0;

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
        on_click: impl Fn(&mut App) + 'static,
    ) -> impl IntoElement {
        IconButton::new(id.into(), icon)
            .style(ButtonStyle::Subtle)
            .size(ButtonSize::Large)
            .icon_size(IconSize::XSmall)
            .on_click(move |_, _, cx| on_click(cx))
    }

    /// The album-art slot when there is no art to show, or the user turned
    /// covers off.
    fn render_cover_placeholder(cx: &App) -> impl IntoElement {
        h_flex()
            .size(px(COVER_SIZE))
            .justify_center()
            .rounded(Radius::Small.pixels())
            .bg(cx.theme().colors.elevated_surface_background)
            .border_1()
            .border_color(cx.theme().colors.border_variant)
            .child(
                Icon::new(IconName::Music)
                    .size(IconSize::XSmall)
                    .color(Color::Default),
            )
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
        let volume_value = player.volume.unwrap_or(0.0);
        let duration = Self::format_duration(player.duration_us);

        let can_control = player.can_control;
        let service_short = service_name
            .rsplit('.')
            .next()
            .unwrap_or("player")
            .to_string();

        let cover = show_cover
            .then(|| player.art_url.clone())
            .flatten()
            .map(|source| {
                div()
                    .size(px(COVER_SIZE))
                    .rounded(Radius::Small.pixels())
                    .overflow_hidden()
                    .border_1()
                    .border_color(theme.colors.border_variant)
                    .child(img(source).size_full())
                    .into_any_element()
            })
            .unwrap_or_else(|| Self::render_cover_placeholder(cx).into_any_element());

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

        v_flex()
            .w_full()
            .p(Spacing::Medium.pixels())
            .panel_card(cx)
            .gap(Spacing::Medium.pixels())
            .child(
                h_flex()
                    .w_full()
                    .justify_between()
                    .gap(Spacing::Medium.pixels())
                    .child(
                        h_flex().gap(Spacing::Medium.pixels()).child(cover).child(
                            v_flex()
                                .overflow_hidden()
                                .child(
                                    Label::new(title)
                                        .size(TextSize::Small)
                                        .weight(FontWeight::MEDIUM)
                                        .truncate(),
                                )
                                .child(Label::new(subtitle).size(TextSize::XSmall).truncate()),
                        ),
                    )
                    .child(
                        Label::new(format!("{}  {}", Self::status_text(player.state), volume))
                            .size(TextSize::XSmall)
                            .color(Color::Custom(status_color)),
                    ),
            )
            .when(player.duration_us.is_some(), |el| {
                el.child(
                    h_flex()
                        .w_full()
                        .gap(Spacing::XSmall.pixels())
                        .child(
                            Icon::new(IconName::Clock)
                                .size(IconSize::XSmall)
                                .color(Color::Muted),
                        )
                        .child(
                            Label::new(duration)
                                .size(TextSize::XSmall)
                                .color(Color::Muted),
                        ),
                )
            })
            .child(
                h_flex()
                    .w_full()
                    .justify_between()
                    .gap(Spacing::XSmall.pixels())
                    .child(
                        Label::new(service_short)
                            .size(TextSize::XSmall)
                            .color(Color::Muted),
                    )
                    .child(
                        h_flex()
                            .gap(Spacing::XSmall.pixels())
                            .when(can_control, |el| {
                                el.child(Self::render_control_button(
                                    format!("mpris-prev-{}", service_name),
                                    IconName::SkipBack,
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
                                    move |cx| {
                                        Self::run_command(
                                            cx,
                                            dec_sub.clone(),
                                            dec_service.clone(),
                                            PlayerCommand::Volume(volume_value - 5.0),
                                        );
                                    },
                                ))
                                .child(
                                    Self::render_control_button(
                                        format!("mpris-inc-{}", inc_service),
                                        IconName::Plus,
                                        move |cx| {
                                            Self::run_command(
                                                cx,
                                                inc_sub.clone(),
                                                inc_service.clone(),
                                                PlayerCommand::Volume(volume_value + 5.0),
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
                v_flex()
                    .w_full()
                    .h_full()
                    .gap(Spacing::Large.pixels())
                    .child(
                        h_flex()
                            .gap(Spacing::Medium.pixels())
                            .child(
                                Icon::new(IconName::Volume)
                                    .size(IconSize::Large)
                                    .color(Color::Default),
                            )
                            .child(
                                Label::new("Media Players")
                                    .size(TextSize::Large)
                                    .weight(FontWeight::BOLD),
                            ),
                    )
                    .when(is_empty, |el| {
                        el.child(
                            h_flex().flex_1().justify_center().child(
                                Label::new("No MPRIS players detected")
                                    .size(TextSize::Small)
                                    .color(Color::Muted),
                            ),
                        )
                    })
                    .when(!is_empty, |el| {
                        el.child(
                            v_flex()
                                .flex_1()
                                .overflow_hidden()
                                .gap(Spacing::Medium.pixels())
                                .children(players.into_iter().map(|player| {
                                    self.render_player_card(player, cx).into_any_element()
                                })),
                        )
                    }),
            )
    }
}

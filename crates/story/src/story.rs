//! Story gallery - a per-component showcase browser with sidebar navigation
//! and Base16 scheme switching.
//!
//! This is where the component set gets looked at. The shell itself is a
//! Wayland layer-shell client scattered across a bar, a dock and half a
//! dozen transient popups, which makes "does this widget actually look
//! right under this palette" nearly impossible to answer in situ. The
//! gallery is a plain window that renders every component against the live
//! theme, so palette work and component work can both be seen.
//!
//! Run with: `cargo run -p story`

#![allow(
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::redundant_clone,
    unused_qualifications,
    unreachable_pub
)]

mod assets;
mod layout;
mod schemes;
mod stories;

use assets::ComposedAssets;

/// Re-exports for story files - each story just writes `use crate::prelude::*`.
pub mod prelude {
    pub use gpui::{
        AnyView, App, AppContext, Context, Entity, InteractiveElement, IntoElement, ParentElement,
        Render, SharedString, StatefulInteractiveElement, Styled, WeakEntity, Window,
        prelude::FluentBuilder, px,
    };
    pub use ui::*;

    pub use crate::layout::{example, example_group};
}

use std::ops::DerefMut;

use gpui::{Bounds, Pixels, WindowBounds, WindowOptions, size};
use gpui_platform::application;
use prelude::*;
use schemes::schemes;
use strum::{Display, EnumIter, IntoEnumIterator};

// ---------------------------------------------------------------------------
// Story registry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, Display)]
pub enum StoryCategory {
    Typography,
    #[strum(serialize = "Icons & Images")]
    IconsAndImages,
    Buttons,
    Inputs,
    #[strum(serialize = "Data Display")]
    DataDisplay,
    Feedback,
    Navigation,
    Layout,
    /// Composite shell surfaces from `ui::patterns` - bespoke, not
    /// primitives. Last in the sidebar, and kept in its own group, so the
    /// component set above it stays readable as a component set.
    #[strum(serialize = "Shell Patterns")]
    Patterns,
}

pub struct StoryEntry {
    pub name: &'static str,
    pub category: StoryCategory,
    pub build: fn(&mut Window, &mut App) -> AnyView,
}

pub static STORIES: &[StoryEntry] = &[
    // Typography
    StoryEntry {
        name: "Label",
        category: StoryCategory::Typography,
        build: stories::label::build,
    },
    StoryEntry {
        name: "Headline",
        category: StoryCategory::Typography,
        build: stories::headline::build,
    },
    StoryEntry {
        name: "HighlightedLabel",
        category: StoryCategory::Typography,
        build: stories::highlighted_label::build,
    },
    // Icons & Images
    StoryEntry {
        name: "Icon",
        category: StoryCategory::IconsAndImages,
        build: stories::icon::build,
    },
    StoryEntry {
        name: "DecoratedIcon",
        category: StoryCategory::IconsAndImages,
        build: stories::decorated_icon::build,
    },
    StoryEntry {
        name: "Avatar",
        category: StoryCategory::IconsAndImages,
        build: stories::avatar::build,
    },
    // Buttons
    StoryEntry {
        name: "Button",
        category: StoryCategory::Buttons,
        build: stories::button::build,
    },
    StoryEntry {
        name: "IconButton",
        category: StoryCategory::Buttons,
        build: stories::icon_button::build,
    },
    StoryEntry {
        name: "ButtonLink",
        category: StoryCategory::Buttons,
        build: stories::button_link::build,
    },
    StoryEntry {
        name: "SplitButton",
        category: StoryCategory::Buttons,
        build: stories::split_button::build,
    },
    StoryEntry {
        name: "CopyButton",
        category: StoryCategory::Buttons,
        build: stories::copy_button::build,
    },
    StoryEntry {
        name: "ToggleButtonGroup",
        category: StoryCategory::Buttons,
        build: stories::toggle_button::build,
    },
    // Inputs
    StoryEntry {
        name: "Checkbox",
        category: StoryCategory::Inputs,
        build: stories::checkbox::build,
    },
    StoryEntry {
        name: "Radio",
        category: StoryCategory::Inputs,
        build: stories::radio::build,
    },
    StoryEntry {
        name: "Slider",
        category: StoryCategory::Inputs,
        build: stories::slider::build,
    },
    StoryEntry {
        name: "Stepper",
        category: StoryCategory::Inputs,
        build: stories::stepper::build,
    },
    StoryEntry {
        name: "Switch",
        category: StoryCategory::Inputs,
        build: stories::switch::build,
    },
    StoryEntry {
        name: "TextField",
        category: StoryCategory::Inputs,
        build: stories::text_field::build,
    },
    StoryEntry {
        name: "Disclosure",
        category: StoryCategory::Inputs,
        build: stories::disclosure::build,
    },
    StoryEntry {
        name: "DropdownMenu",
        category: StoryCategory::Inputs,
        build: stories::dropdown_menu::build,
    },
    // Data Display
    StoryEntry {
        name: "List",
        category: StoryCategory::DataDisplay,
        build: stories::list::build,
    },
    StoryEntry {
        name: "VirtualList",
        category: StoryCategory::DataDisplay,
        build: stories::virtual_list::build,
    },
    StoryEntry {
        name: "VariableList",
        category: StoryCategory::DataDisplay,
        build: stories::variable_list::build,
    },
    StoryEntry {
        name: "TreeView",
        category: StoryCategory::DataDisplay,
        build: stories::tree_view::build,
    },
    StoryEntry {
        name: "Progress",
        category: StoryCategory::DataDisplay,
        build: stories::progress::build,
    },
    StoryEntry {
        name: "Indicator",
        category: StoryCategory::DataDisplay,
        build: stories::indicator::build,
    },
    StoryEntry {
        name: "Chip",
        category: StoryCategory::DataDisplay,
        build: stories::chip::build,
    },
    StoryEntry {
        name: "DescriptionList",
        category: StoryCategory::DataDisplay,
        build: stories::description_list::build,
    },
    StoryEntry {
        name: "KeyBinding",
        category: StoryCategory::DataDisplay,
        build: stories::keybinding::build,
    },
    StoryEntry {
        name: "KeybindingHint",
        category: StoryCategory::DataDisplay,
        build: stories::keybinding_hint::build,
    },
    // Feedback
    StoryEntry {
        name: "Banner",
        category: StoryCategory::Feedback,
        build: stories::banner::build,
    },
    StoryEntry {
        name: "Callout",
        category: StoryCategory::Feedback,
        build: stories::callout::build,
    },
    StoryEntry {
        name: "Skeleton",
        category: StoryCategory::Feedback,
        build: stories::skeleton::build,
    },
    StoryEntry {
        name: "Spinner",
        category: StoryCategory::Feedback,
        build: stories::spinner::build,
    },
    StoryEntry {
        name: "HoverCard",
        category: StoryCategory::Feedback,
        build: stories::hover_card::build,
    },
    StoryEntry {
        name: "Tooltip",
        category: StoryCategory::Feedback,
        build: stories::tooltip::build,
    },
    // Navigation
    StoryEntry {
        name: "Breadcrumb",
        category: StoryCategory::Navigation,
        build: stories::breadcrumb::build,
    },
    StoryEntry {
        name: "Pagination",
        category: StoryCategory::Navigation,
        build: stories::pagination::build,
    },
    StoryEntry {
        name: "Tab",
        category: StoryCategory::Navigation,
        build: stories::tab::build,
    },
    StoryEntry {
        name: "Menu",
        category: StoryCategory::Navigation,
        build: stories::menu::build,
    },
    // Layout
    StoryEntry {
        name: "Accordion",
        category: StoryCategory::Layout,
        build: stories::accordion::build,
    },
    StoryEntry {
        name: "Divider",
        category: StoryCategory::Layout,
        build: stories::divider::build,
    },
    StoryEntry {
        name: "GradientFade",
        category: StoryCategory::Layout,
        build: stories::gradient_fade::build,
    },
    StoryEntry {
        name: "Modal",
        category: StoryCategory::Layout,
        build: stories::modal::build,
    },
    StoryEntry {
        name: "Popover",
        category: StoryCategory::Layout,
        build: stories::popover::build,
    },
    StoryEntry {
        name: "Sheet",
        category: StoryCategory::Layout,
        build: stories::sheet::build,
    },
    StoryEntry {
        name: "Squircle",
        category: StoryCategory::Layout,
        build: stories::squircle::build,
    },
    StoryEntry {
        name: "Motion",
        category: StoryCategory::Layout,
        build: stories::motion::build,
    },
    // Shell Patterns
    StoryEntry {
        name: "Launcher",
        category: StoryCategory::Patterns,
        build: stories::launcher::build,
    },
    StoryEntry {
        name: "Bar",
        category: StoryCategory::Patterns,
        build: stories::bar::build,
    },
    StoryEntry {
        name: "Panel",
        category: StoryCategory::Patterns,
        build: stories::panel::build,
    },
    StoryEntry {
        name: "OSD",
        category: StoryCategory::Patterns,
        build: stories::osd::build,
    },
];

// ---------------------------------------------------------------------------
// Gallery
// ---------------------------------------------------------------------------

struct Gallery {
    selected_index: usize,
    current_view: Option<AnyView>,
    scroll: gpui::ScrollHandle,
    /// `STORY_SCROLL=600`: start the content pane scrolled down, for a
    /// screenshot of an example further down the page.
    scroll_to: Option<Pixels>,
}

impl Gallery {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let selected_index = initial_story();
        Self {
            selected_index,
            current_view: Some((STORIES[selected_index].build)(window, cx.deref_mut())),
            scroll: gpui::ScrollHandle::new(),
            scroll_to: std::env::var("STORY_SCROLL")
                .ok()
                .and_then(|v| v.parse().ok())
                .map(px),
        }
    }

    fn select_story(&mut self, index: usize, cx: &mut Context<Self>) {
        if index != self.selected_index && index < STORIES.len() {
            self.selected_index = index;
            self.current_view = None;
            cx.notify();
        }
    }
}

impl Render for Gallery {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Lazily rebuild the story view when the selection changed.
        if self.current_view.is_none() {
            self.current_view = Some((STORIES[self.selected_index].build)(window, cx.deref_mut()));
        }
        let view = self.current_view.clone().unwrap();
        let colors = cx.theme().colors();
        let translucent = cx.theme().opacity < 1.0;
        let weak = cx.entity().downgrade();
        if let Some(offset) = self.scroll_to {
            // Every frame, since the first layout clamps it to an empty page.
            self.scroll.set_offset(gpui::point(px(0.), -offset));
        }

        h_flex()
            .size_full()
            .bg(colors.background)
            // ---- Sidebar ----
            .child(
                v_flex()
                    .w(px(240.0))
                    .h_full()
                    .flex_shrink_0()
                    .border_r_1()
                    .border_color(colors.border)
                    .bg(colors.surface_background)
                    .child(
                        v_flex()
                            .p(Spacing::Medium.pixels())
                            .gap(Spacing::Medium.pixels())
                            .child(Headline::new("gpui-shell components").size(HeadlineSize::Small))
                            .child(self.render_theme_switcher(cx))
                            .child(self.render_shape_controls(cx))
                            .child(Divider::horizontal()),
                    )
                    .child(
                        v_flex()
                            .id("sidebar-scroll")
                            .flex_1()
                            .overflow_y_scroll()
                            .p(Spacing::Medium.pixels())
                            .pt_0()
                            .gap(Spacing::Small.pixels())
                            .children(self.render_sidebar_groups(&weak)),
                    ),
            )
            // ---- Content pane ----
            .child(
                gpui::div()
                    .relative()
                    .flex_1()
                    .h_full()
                    .children(translucent.then(|| backdrop(cx)))
                    .child(
                        v_flex()
                            .id("content-scroll")
                            .size_full()
                            .overflow_y_scroll()
                            .track_scroll(&self.scroll)
                            .p(Spacing::XXLarge.pixels())
                            .gap(Spacing::Large.pixels())
                            .child(
                                Headline::new(STORIES[self.selected_index].name)
                                    .size(HeadlineSize::Medium),
                            )
                            .child(Divider::horizontal())
                            .child(view),
                    ),
            )
    }
}

/// Something loud behind the content, so a translucent surface has
/// something to be translucent over.
fn backdrop(cx: &App) -> impl IntoElement {
    let colors = cx.theme().colors();
    let status = &colors.status;
    let blob = |color: gpui::Hsla, size: f32, top: f32, left: f32| {
        gpui::div()
            .absolute()
            .top(px(top))
            .left(px(left))
            .size(px(size))
            .rounded(px(size / 2.))
            .bg(color)
    };
    gpui::div()
        .absolute()
        .inset_0()
        .overflow_hidden()
        .bg(gpui::linear_gradient(
            135.,
            gpui::linear_color_stop(colors.accent, 0.),
            gpui::linear_color_stop(status.error, 1.),
        ))
        .child(blob(status.success, 260., 40., 120.))
        .child(blob(status.warning, 180., 220., 420.))
        .child(blob(colors.text, 140., 90., 640.))
        .child(blob(status.info, 320., 320., 180.))
        .children((0..14).map(|i| {
            gpui::div()
                .absolute()
                .top_0()
                .bottom_0()
                .left(px(i as f32 * 90.))
                .w(px(12.))
                .bg(colors.background)
        }))
        // A light wash keeps the story captions legible over the noise.
        .child(
            gpui::div()
                .absolute()
                .inset_0()
                .bg(colors.background.fade(Opacity::Subtle)),
        )
}

impl Gallery {
    fn render_sidebar_groups(&self, weak: &gpui::WeakEntity<Self>) -> Vec<impl IntoElement> {
        let mut groups = Vec::new();

        for category in StoryCategory::iter() {
            let entries: Vec<(usize, &StoryEntry)> = STORIES
                .iter()
                .enumerate()
                .filter(|(_, s)| s.category == category)
                .collect();

            if entries.is_empty() {
                continue;
            }

            let mut group = v_flex().gap(Spacing::XSmall.pixels()).child(
                Label::new(category.to_string())
                    .size(TextSize::XSmall)
                    .color(Color::Muted),
            );

            for (index, entry) in entries {
                let is_selected = index == self.selected_index;
                let weak = weak.clone();
                group = group.child(
                    ListItem::new(SharedString::from(format!("story-{}", entry.name)))
                        .child(Label::new(entry.name))
                        .toggle_state(is_selected)
                        .inset(true)
                        .spacing(ListItemSpacing::Dense)
                        .on_click(move |_event, _window, cx| {
                            weak.update(cx, |this, cx| {
                                this.select_story(index, cx);
                            })
                            .ok();
                        }),
                );
            }

            groups.push(group);
        }
        groups
    }

    /// Scheme picker. Swaps the Base16 palette in place, which is exactly
    /// what the shell does when the user picks a scheme - so what the
    /// gallery shows is what the bar will show.
    fn render_theme_switcher(&self, cx: &Context<Self>) -> impl IntoElement {
        let current = cx.theme().name.clone();
        let weak = cx.entity().downgrade();

        h_flex()
            .gap(Spacing::XSmall.pixels())
            .flex_wrap()
            .children(schemes().iter().map(move |scheme| {
                let is_current = scheme.name == current;
                let weak = weak.clone();
                let name = scheme.name.clone();
                let palette = scheme.palette;
                Button::new(
                    SharedString::from(format!("theme-{}", scheme.name)),
                    scheme.name.clone(),
                )
                .size(ButtonSize::Compact)
                .style(if is_current {
                    ButtonStyle::Filled
                } else {
                    ButtonStyle::Subtle
                })
                .toggle_state(is_current)
                .on_click(move |_event, _window, cx| {
                    let mut theme = cx.theme().clone();
                    theme.set_palette(name.clone(), palette);
                    Theme::set(theme, cx);
                    weak.update(cx, |_, cx| cx.notify()).ok();
                })
            }))
    }

    /// Radius scale and surface opacity, the two theme knobs that are not
    /// colours. Every pattern and component should survive each stop.
    fn render_shape_controls(&self, cx: &Context<Self>) -> impl IntoElement {
        const SCALES: [f32; 5] = [0.0, 0.5, 1.0, 1.5, 2.0];
        const OPACITIES: [f32; 4] = [1.0, 0.85, 0.7, 0.5];
        let theme = cx.theme();
        let (scale, opacity) = (theme.radius_scale, theme.opacity);
        let weak = cx.entity().downgrade();

        let row = |label: &'static str, buttons: Vec<Button>| {
            h_flex()
                .gap(Spacing::XSmall.pixels())
                .items_center()
                .child(
                    Label::new(label)
                        .size(TextSize::XSmall)
                        .color(Color::Muted)
                        .into_element(),
                )
                .children(buttons)
        };
        let button =
            |id: String, label: String, current: bool, apply: fn(&mut Theme, f32), value: f32| {
                let weak = weak.clone();
                Button::new(SharedString::from(id), label)
                    .size(ButtonSize::Compact)
                    .style(if current {
                        ButtonStyle::Filled
                    } else {
                        ButtonStyle::Subtle
                    })
                    .toggle_state(current)
                    .on_click(move |_event, _window, cx| {
                        let mut theme = cx.theme().clone();
                        apply(&mut theme, value);
                        Theme::set(theme, cx);
                        weak.update(cx, |_, cx| cx.notify()).ok();
                    })
            };

        v_flex()
            .gap(Spacing::XSmall.pixels())
            .child(row(
                "Radius",
                SCALES
                    .iter()
                    .map(|value| {
                        button(
                            format!("radius-{value}"),
                            format!("{value}"),
                            (scale - value).abs() < f32::EPSILON,
                            Theme::set_radius_scale,
                            *value,
                        )
                    })
                    .collect(),
            ))
            .child(row(
                "Opacity",
                OPACITIES
                    .iter()
                    .map(|value| {
                        button(
                            format!("opacity-{value}"),
                            format!("{}%", (value * 100.) as u32),
                            (opacity - value).abs() < f32::EPSILON,
                            Theme::set_opacity,
                            *value,
                        )
                    })
                    .collect(),
            ))
    }
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

/// Startup state from the environment, so a screenshot run needs no clicks:
/// `STORY=Bar STORY_RADIUS_SCALE=0 STORY_OPACITY=0.7 STORY_SCHEME="Gruvbox Dark"`.
fn apply_env(cx: &mut App) {
    let mut theme = cx.theme().clone();
    if let Some(scheme) = std::env::var("STORY_SCHEME").ok().and_then(|name| {
        schemes()
            .iter()
            .find(|s| s.name.eq_ignore_ascii_case(&name))
    }) {
        theme.set_palette(scheme.name.clone(), scheme.palette);
    }
    if let Some(scale) = std::env::var("STORY_RADIUS_SCALE")
        .ok()
        .and_then(|v| v.parse().ok())
    {
        theme.set_radius_scale(scale);
    }
    if let Some(opacity) = std::env::var("STORY_OPACITY")
        .ok()
        .and_then(|v| v.parse().ok())
    {
        theme.set_opacity(opacity);
    }
    Theme::set(theme, cx);
}

/// `STORY_SIZE=1100x1150` for a taller window when a story does not fit.
fn window_size() -> gpui::Size<Pixels> {
    std::env::var("STORY_SIZE")
        .ok()
        .and_then(|value| {
            let (w, h) = value.split_once('x')?;
            Some(size(px(w.parse().ok()?), px(h.parse().ok()?)))
        })
        .unwrap_or_else(|| size(px(1100.0), px(760.0)))
}

/// The story `STORY` names, or the first one.
fn initial_story() -> usize {
    std::env::var("STORY")
        .ok()
        .and_then(|name| {
            STORIES
                .iter()
                .position(|story| story.name.eq_ignore_ascii_case(&name))
        })
        .unwrap_or(0)
}

fn main() {
    application()
        .with_assets(ComposedAssets)
        .run(|cx: &mut App| {
            Theme::init(cx);
            ui::init(cx);
            apply_env(cx);

            let bounds = Bounds::centered(None, window_size(), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    app_id: Some("story".into()),
                    ..Default::default()
                },
                |window, cx| {
                    window.set_rem_size(cx.theme().font_size);
                    cx.new(|cx| Gallery::new(window, cx))
                },
            )
            .unwrap();

            cx.activate(true);
        });
}

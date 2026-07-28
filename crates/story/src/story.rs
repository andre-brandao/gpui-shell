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

use gpui::{Bounds, WindowBounds, WindowOptions, size};
use gpui_platform::application;
use prelude::*;
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
];

// ---------------------------------------------------------------------------
// Gallery
// ---------------------------------------------------------------------------

struct Gallery {
    selected_index: usize,
    current_view: Option<AnyView>,
}

impl Gallery {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let selected_index = 0;
        Self {
            selected_index,
            current_view: Some((STORIES[selected_index].build)(window, cx.deref_mut())),
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
        let weak = cx.entity().downgrade();

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
                v_flex()
                    .id("content-scroll")
                    .flex_1()
                    .h_full()
                    .overflow_y_scroll()
                    .p(Spacing::XXLarge.pixels())
                    .gap(Spacing::Large.pixels())
                    .child(
                        Headline::new(STORIES[self.selected_index].name).size(HeadlineSize::Medium),
                    )
                    .child(Divider::horizontal())
                    .child(view),
            )
    }
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
            .children(schemes().into_iter().map(move |scheme| {
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
}

// ---------------------------------------------------------------------------
// Schemes
// ---------------------------------------------------------------------------

/// Palettes offered by the switcher.
///
/// A deliberately varied set: the whole point of deriving ~50 tokens from
/// 16 colors is that the derivation has to hold up on palettes it was not
/// tuned against. A light scheme and a low-contrast one catch far more than
/// another dark grey would.
fn schemes() -> Vec<ThemeScheme> {
    let mut schemes = builtin_schemes();
    for (name, description, colors) in EXTRA_SCHEMES {
        match Base16Palette::from_hex(colors) {
            Ok(palette) => schemes.push(ThemeScheme::new(*name, *description, palette)),
            // A malformed palette here is a typo in this file, not user
            // input - surface it rather than silently showing fewer schemes.
            Err(err) => eprintln!("story: scheme `{name}` is malformed: {err}"),
        }
    }
    schemes
}

type SchemeSpec = (&'static str, &'static str, &'static [&'static str; 16]);

static EXTRA_SCHEMES: &[SchemeSpec] = &[
    (
        "Gruvbox Dark",
        "Warm, medium contrast",
        &[
            "#282828", "#3c3836", "#504945", "#665c54", "#bdae93", "#d5c4a1", "#ebdbb2", "#fbf1c7",
            "#fb4934", "#fe8019", "#fabd2f", "#b8bb26", "#8ec07c", "#83a598", "#d3869b", "#d65d0e",
        ],
    ),
    (
        "Solarized Light",
        "Light scheme - checks the derived tints invert correctly",
        &[
            "#fdf6e3", "#eee8d5", "#93a1a1", "#839496", "#657b83", "#586e75", "#073642", "#002b36",
            "#dc322f", "#cb4b16", "#b58900", "#859900", "#2aa198", "#268bd2", "#6c71c4", "#d33682",
        ],
    ),
    (
        "Nord",
        "Low contrast - stresses the subtle border steps",
        &[
            "#2e3440", "#3b4252", "#434c5e", "#4c566a", "#d8dee9", "#e5e9f0", "#eceff4", "#8fbcbb",
            "#bf616a", "#d08770", "#ebcb8b", "#a3be8c", "#88c0d0", "#81a1c1", "#b48ead", "#5e81ac",
        ],
    ),
];

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    application()
        .with_assets(ComposedAssets)
        .run(|cx: &mut App| {
            Theme::init(cx);
            ui::init(cx);

            let bounds = Bounds::centered(None, size(px(1100.0), px(760.0)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
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

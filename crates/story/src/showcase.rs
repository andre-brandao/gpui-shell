//! Single-page showcase - every interactive component wired up to real
//! state, so you can click checkboxes, flip switches, drag sliders, expand
//! the disclosure and select list rows.
//!
//! Complements the `story` gallery rather than duplicating it. The gallery
//! isolates one component at a time with every variant laid out, which is
//! what you want when working *on* a component. This page shows them all at
//! once against a shared surface, which is what you want when judging
//! whether a palette holds together - contrast collisions between
//! neighbouring components only show up when they're neighbours.
//!
//! Run with: `cargo run -p story --bin showcase`.
//!
//! # A note on frame rate
//!
//! gpui only renders when something asks it to, and a continuously animating
//! component asks on every frame - `Animation` calls
//! `Window::request_animation_frame`, which notifies whichever view was
//! rendering when the element was built. Every animated element on this page
//! is built inside `Showcase::render`, so a single spinner dirties the whole
//! page.
//!
//! What that costs is *not* rebuilding the cards, which is a small fraction
//! of a frame. Almost all of it is what gpui does after `render` returns:
//! `Window::draw` clears the taffy layout engine every frame, so the entire
//! tree is re-laid-out, re-prepainted and re-painted from scratch. The cost
//! scales with element count, not pixels, so shrinking the window does not
//! help.
//!
//! Two things would actually move the needle, neither of them done here:
//!
//! - `opt-level = "z"` in the workspace release profile. Building at
//!   `opt-level = 3` measurably cuts frame time for a larger binary; Zed's
//!   own release profile leaves `opt-level` at its default of 3.
//! - `Entity::cached(style)`. A cached child view whose entity is not in
//!   `dirty_views` skips render/layout/prepaint entirely via
//!   `Window::reuse_prepaint`, so an animation in one corner stops
//!   re-laying-out everything else. This is how Zed keeps its spinners cheap:
//!   panes and docks are cached view boundaries (`pane_group.rs`,
//!   `dock.rs`). It needs a definite size per cached view, which is why the
//!   cards here are plain elements instead.
//!
//! So the animated cards are gated behind a toggle. Off - the default - the
//! page requests no frames at all and sits idle, which is what you want from
//! a window left open next to an editor. On, the spinners spin and the page
//! pays for it.

#![allow(
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::redundant_clone,
    unreachable_pub
)]

use std::cell::Cell;
use std::rc::Rc;

use gpui::{
    App, AppContext, Bounds, Context, Entity, FocusHandle, ImageSource, InteractiveElement,
    IntoElement, ParentElement, Pixels, Render, SharedString, StatefulInteractiveElement, Styled,
    WeakEntity, Window, WindowBounds, WindowOptions, canvas, div, prelude::FluentBuilder, px, size,
};
use gpui_platform::application;
use ui::*;

mod schemes;
use schemes::schemes;

const DEMO_IMAGE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/demo/avatar.jpg");

struct Showcase {
    /// Gates the continuously animating cards. See the module docs.
    animations_on: bool,
    checkbox_small: ToggleState,
    checkbox_default: ToggleState,
    checkbox_large: ToggleState,
    checkbox_tri: ToggleState,
    switch_notifications: ToggleState,
    switch_autosave: ToggleState,
    switch_telemetry: ToggleState,
    button_pin_toggled: bool,
    icon_button_starred: bool,
    disclosure_open: bool,
    selected_nav: SharedString,
    selected_tab: usize,
    banner_visible: bool,
    modal_open: bool,
    modal_focus: FocusHandle,
    menu_open: bool,
    menu: Entity<Menu>,
    menu_trigger_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    submitted_value: SharedString,
    text_field: Entity<TextField>,
    radio_selected: usize,
    slider_basic: f32,
    slider_stepped: f32,
    stepper_value: f32,
    accordion_expanded: [bool; 3],
    progress_value: f32,
    pagination_page: u32,
    demo_image: ImageSource,
}

impl Showcase {
    fn new(cx: &mut Context<Self>) -> Self {
        let weak = cx.entity().downgrade();
        let text_field = cx.new(|cx| {
            TextField::with_value(cx, "Hello, gpui-shell")
                .placeholder("Type something...")
                .on_submit(move |value, _window, cx| {
                    let value = SharedString::from(value.to_string());
                    weak.update(cx, |this, cx| {
                        this.submitted_value = value;
                        cx.notify();
                    })
                    .ok();
                })
        });
        let menu = cx.new(|cx| {
            Menu::new(cx)
                .header("File")
                .entry_with_icon("menu-new", IconName::Plus, "New File", |_, _| {})
                .keybinding_entry("menu-save", "Save", ["Ctrl", "S"], |_, _| {})
                .keybinding_entry(
                    "menu-saveas",
                    "Save As...",
                    ["Ctrl", "Shift", "S"],
                    |_, _| {},
                )
                .separator()
                .header("Edit")
                .entry("menu-cut", "Cut", |_, _| {})
                .entry("menu-copy", "Copy", |_, _| {})
                .entry("menu-paste", "Paste", |_, _| {})
                .separator()
                .disabled_entry("menu-disabled", "Unavailable")
        });
        cx.subscribe(&menu, |this, _, _: &gpui::DismissEvent, cx| {
            this.menu_open = false;
            cx.notify();
        })
        .detach();
        Self {
            animations_on: false,
            checkbox_small: ToggleState::Selected,
            checkbox_default: ToggleState::Unselected,
            checkbox_large: ToggleState::Selected,
            checkbox_tri: ToggleState::Indeterminate,
            switch_notifications: ToggleState::Selected,
            switch_autosave: ToggleState::Unselected,
            switch_telemetry: ToggleState::Selected,
            button_pin_toggled: false,
            icon_button_starred: true,
            disclosure_open: true,
            selected_nav: SharedString::from("nav-search"),
            selected_tab: 0,
            banner_visible: true,
            modal_open: false,
            modal_focus: cx.focus_handle(),
            menu_open: false,
            menu,
            menu_trigger_bounds: Rc::new(Cell::new(None)),
            submitted_value: SharedString::default(),
            text_field,
            radio_selected: 1,
            slider_basic: 60.0,
            slider_stepped: 30.0,
            stepper_value: 3.0,
            accordion_expanded: [true, false, false],
            progress_value: 65.0,
            pagination_page: 3,
            demo_image: std::path::PathBuf::from(DEMO_IMAGE).into(),
        }
    }
}

fn toggle_setter<F>(
    weak: &WeakEntity<Showcase>,
    set: F,
) -> impl Fn(&ToggleState, &mut Window, &mut App) + 'static
where
    F: Fn(&mut Showcase, ToggleState) + 'static,
{
    let weak = weak.clone();
    move |state, _window, cx| {
        let state = *state;
        weak.update(cx, |this, cx| {
            set(this, state);
            cx.notify();
        })
        .ok();
    }
}

fn bool_toggle<F>(
    weak: &WeakEntity<Showcase>,
    pick: F,
) -> impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static
where
    F: Fn(&mut Showcase) -> &mut bool + 'static,
{
    let weak = weak.clone();
    move |_event, _window, cx| {
        weak.update(cx, |this, cx| {
            let field = pick(this);
            *field = !*field;
            cx.notify();
        })
        .ok();
    }
}

fn set_field<T, F>(
    weak: &WeakEntity<Showcase>,
    value: T,
    pick: F,
) -> impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static
where
    T: Clone + 'static,
    F: Fn(&mut Showcase) -> &mut T + 'static,
{
    let weak = weak.clone();
    move |_event, _window, cx| {
        let value = value.clone();
        weak.update(cx, |this, cx| {
            *pick(this) = value;
            cx.notify();
        })
        .ok();
    }
}

impl Render for Showcase {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = *cx.theme().colors();
        let weak = cx.entity().downgrade();

        let current_theme = cx.theme().name.clone();

        let header = h_flex()
            .w_full()
            .items_center()
            .justify_between()
            .px(Spacing::Large.pixels())
            .py(Spacing::Medium.pixels())
            .child(
                h_flex()
                    .items_center()
                    .gap(Spacing::Small.pixels())
                    .child(
                        Icon::new(IconName::Layers)
                            .size(IconSize::XLarge)
                            .color(Color::Accent),
                    )
                    .child(Headline::new("gpui-shell showcase").size(HeadlineSize::Medium)),
            )
            .child(
                h_flex().gap(Spacing::Small.pixels()).flex_wrap().child(
                    Switch::new("toggle-animations", self.animations_on)
                        .size(SwitchSize::Small)
                        .label("Animations")
                        .on_click({
                            let weak = weak.clone();
                            move |state, _window, cx| {
                                let on = state.selected();
                                weak.update(cx, |this, cx| {
                                    this.animations_on = on;
                                    cx.notify();
                                })
                                .ok();
                            }
                        }),
                ),
            )
            .child(h_flex().gap(Spacing::Small.pixels()).flex_wrap().children(
                schemes().iter().map(|scheme| {
                    let is_current = scheme.name == current_theme;
                    let name = scheme.name.clone();
                    let palette = scheme.palette;
                    Button::new(
                        SharedString::from(format!("btn-theme-{}", scheme.name)),
                        scheme.name.clone(),
                    )
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
                    })
                }),
            ));

        let cards: Vec<gpui::AnyElement> = vec![
            // -------- Typography --------
            card(
                "Labels",
                v_flex()
                    .gap(Spacing::XSmall.pixels())
                    .child(
                        h_flex()
                            .gap(Spacing::Medium.pixels())
                            .items_baseline()
                            .child(Label::new("XSmall").size(TextSize::XSmall))
                            .child(Label::new("Small").size(TextSize::Small))
                            .child(Label::new("Default"))
                            .child(Label::new("Large").size(TextSize::Large)),
                    )
                    .child(
                        h_flex()
                            .gap(Spacing::Small.pixels())
                            .child(Label::new("Default"))
                            .child(Label::new("Muted").color(Color::Muted))
                            .child(Label::new("Accent").color(Color::Accent))
                            .child(Label::new("Success").color(Color::Success))
                            .child(Label::new("Warning").color(Color::Warning))
                            .child(Label::new("Error").color(Color::Error)),
                    )
                    .child(
                        h_flex()
                            .gap(Spacing::Small.pixels())
                            .child(Label::new("Bold").weight(gpui::FontWeight::BOLD))
                            .child(Label::new("Italic").italic())
                            .child(Label::new("Underline").underline())
                            .child(Label::new("Strike").strikethrough())
                            .child(Label::new("Faded").alpha(0.5)),
                    ),
                colors,
            )
            .into_any_element(),
            card(
                "Headlines",
                v_flex()
                    .gap(Spacing::XSmall.pixels())
                    .child(Headline::new("XSmall").size(HeadlineSize::XSmall))
                    .child(Headline::new("Small").size(HeadlineSize::Small))
                    .child(Headline::new("Medium"))
                    .child(Headline::new("Large").size(HeadlineSize::Large))
                    .child(Headline::new("XLarge").size(HeadlineSize::XLarge)),
                colors,
            )
            .into_any_element(),
            // -------- Icons --------
            card(
                "Icons",
                v_flex()
                    .gap(Spacing::Small.pixels())
                    .child(
                        h_flex()
                            .gap(Spacing::Small.pixels())
                            .child(Icon::new(IconName::Check))
                            .child(Icon::new(IconName::Close))
                            .child(Icon::new(IconName::ChevronDown))
                            .child(Icon::new(IconName::ChevronRight))
                            .child(Icon::new(IconName::Plus))
                            .child(Icon::new(IconName::MagnifyingGlass))
                            .child(Icon::new(IconName::Settings))
                            .child(Icon::new(IconName::Warning).color(Color::Warning))
                            .child(Icon::new(IconName::Info).color(Color::Info))
                            .child(Icon::new(IconName::XCircle).color(Color::Error))
                            .child(Icon::new(IconName::Check).color(Color::Success)),
                    )
                    .child(
                        h_flex()
                            .gap(Spacing::Medium.pixels())
                            .items_center()
                            .child(Icon::new(IconName::Settings).size(IconSize::XSmall))
                            .child(Icon::new(IconName::Settings).size(IconSize::Small))
                            .child(Icon::new(IconName::Settings).size(IconSize::Medium))
                            .child(Icon::new(IconName::Settings).size(IconSize::XLarge)),
                    )
                    // Icon::from_path resolves a path through the active
                    // AssetSource, letting an app mix the shell's curated
                    // catalogue with its own bundled SVGs.
                    .child(
                        h_flex()
                            .gap(Spacing::Small.pixels())
                            .items_center()
                            .child(Icon::from_path("icons/check.svg").color(Color::Accent))
                            .child(Icon::from_path("icons/star.svg").color(Color::Warning))
                            .child(Icon::from_path("icons/heart.svg").color(Color::Error)),
                    ),
                colors,
            )
            .into_any_element(),
            // -------- Buttons --------
            card(
                "Buttons",
                v_flex()
                    .gap(Spacing::XSmall.pixels())
                    .child(
                        h_flex()
                            .gap(Spacing::XSmall.pixels())
                            .flex_wrap()
                            .child(Button::new("btn-filled", "Filled").style(ButtonStyle::Filled))
                            .child(Button::new("btn-subtle", "Subtle").style(ButtonStyle::Subtle))
                            .child(Button::new("btn-ghost", "Ghost").style(ButtonStyle::Ghost))
                            .child(
                                Button::new("btn-outlined", "Outlined")
                                    .style(ButtonStyle::Outlined),
                            )
                            .child(
                                Button::new("btn-transparent", "Transparent")
                                    .style(ButtonStyle::Transparent),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap(Spacing::XSmall.pixels())
                            .flex_wrap()
                            .child(
                                Button::new("btn-tint-accent", "Accent")
                                    .style(ButtonStyle::Tinted(TintColor::Accent)),
                            )
                            .child(
                                Button::new("btn-tint-success", "Success")
                                    .style(ButtonStyle::Tinted(TintColor::Success)),
                            )
                            .child(
                                Button::new("btn-tint-warning", "Warning")
                                    .style(ButtonStyle::Tinted(TintColor::Warning)),
                            )
                            .child(
                                Button::new("btn-tint-error", "Error")
                                    .style(ButtonStyle::Tinted(TintColor::Error)),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap(Spacing::XSmall.pixels())
                            .flex_wrap()
                            .items_center()
                            .child(Button::new("btn-cmp", "Compact").size(ButtonSize::Compact))
                            .child(Button::new("btn-def", "Default"))
                            .child(Button::new("btn-lg", "Large").size(ButtonSize::Large))
                            .child(Button::new("btn-dis", "Disabled").disabled(true))
                            .child(
                                Button::new(
                                    "btn-pin",
                                    if self.button_pin_toggled {
                                        "Pinned"
                                    } else {
                                        "Pin"
                                    },
                                )
                                .icon(IconName::Pin)
                                .style(ButtonStyle::Subtle)
                                .toggle_state(self.button_pin_toggled)
                                .on_click(bool_toggle(&weak, |this| &mut this.button_pin_toggled)),
                            ),
                    ),
                colors,
            )
            .into_any_element(),
            // -------- IconButton + Tooltip --------
            card(
                "IconButton + Tooltip",
                v_flex()
                    .gap(Spacing::Small.pixels())
                    .child(
                        h_flex()
                            .gap(Spacing::Small.pixels())
                            .child(IconButton::new("ib-filled", IconName::Settings))
                            .child(
                                IconButton::new("ib-ghost", IconName::MagnifyingGlass)
                                    .style(ButtonStyle::Subtle),
                            )
                            .child(
                                IconButton::new("ib-out", IconName::Plus)
                                    .style(ButtonStyle::Outlined),
                            )
                            .child(
                                IconButton::new(
                                    "ib-star",
                                    if self.icon_button_starred {
                                        IconName::StarFilled
                                    } else {
                                        IconName::Star
                                    },
                                )
                                .style(ButtonStyle::Subtle)
                                .toggle_state(self.icon_button_starred)
                                .on_click(bool_toggle(&weak, |this| &mut this.icon_button_starred)),
                            )
                            .child(IconButton::new("ib-dis", IconName::Close).disabled(true)),
                    )
                    .child(
                        h_flex()
                            .gap(Spacing::Small.pixels())
                            .child(
                                Button::new("btn-tip-text", "Hover me")
                                    .tooltip(Tooltip::text("This is a tooltip")),
                            )
                            .child(
                                Button::new("btn-tip-meta", "With meta")
                                    .style(ButtonStyle::Subtle)
                                    .tooltip(Tooltip::with_meta("Save file", "Ctrl+S")),
                            )
                            .child(
                                IconButton::new("ib-tip", IconName::Settings)
                                    .style(ButtonStyle::Subtle)
                                    .tooltip(Tooltip::text("Settings")),
                            ),
                    ),
                colors,
            )
            .into_any_element(),
            // -------- Checkbox --------
            card(
                "Checkbox",
                v_flex()
                    .gap(Spacing::XSmall.pixels())
                    .child(
                        Checkbox::new("cb-sm", self.checkbox_small)
                            .size(CheckboxSize::Small)
                            .label("Small")
                            .on_click(toggle_setter(&weak, |this, s| this.checkbox_small = s)),
                    )
                    .child(
                        Checkbox::new("cb-def", self.checkbox_default)
                            .label("Default")
                            .on_click(toggle_setter(&weak, |this, s| this.checkbox_default = s)),
                    )
                    .child(
                        Checkbox::new("cb-lg", self.checkbox_large)
                            .size(CheckboxSize::Large)
                            .label("Large")
                            .on_click(toggle_setter(&weak, |this, s| this.checkbox_large = s)),
                    )
                    .child(
                        Checkbox::new("cb-tri", self.checkbox_tri)
                            .label("Tri-state")
                            .on_click(toggle_setter(&weak, |this, s| this.checkbox_tri = s)),
                    )
                    .child(
                        Checkbox::new("cb-dis", true)
                            .label("Disabled")
                            .disabled(true),
                    ),
                colors,
            )
            .into_any_element(),
            // -------- Switch --------
            card(
                "Switch",
                v_flex()
                    .gap(Spacing::XSmall.pixels())
                    .child(
                        Switch::new("sw-notif", self.switch_notifications)
                            .label("Notifications")
                            .on_click(toggle_setter(&weak, |this, s| {
                                this.switch_notifications = s
                            })),
                    )
                    .child(
                        Switch::new("sw-auto", self.switch_autosave)
                            .label("Auto-save")
                            .on_click(toggle_setter(&weak, |this, s| this.switch_autosave = s)),
                    )
                    .child(
                        Switch::new("sw-tel", self.switch_telemetry)
                            .label("Telemetry")
                            .on_click(toggle_setter(&weak, |this, s| this.switch_telemetry = s)),
                    )
                    .child(
                        Switch::new("sw-dis", false)
                            .label("Disabled")
                            .disabled(true),
                    ),
                colors,
            )
            .into_any_element(),
            // -------- Radio --------
            card(
                "Radio",
                {
                    let options = ["Alpha", "Beta", "Gamma"];
                    let mut col = v_flex().gap(Spacing::XSmall.pixels());
                    for (i, name) in options.iter().enumerate() {
                        let state = if self.radio_selected == i {
                            ToggleState::Selected
                        } else {
                            ToggleState::Unselected
                        };
                        let w = weak.clone();
                        col = col.child(
                            Radio::new(SharedString::from(format!("r-{i}")), state)
                                .label(*name)
                                .on_click(move |_, _, cx| {
                                    w.update(cx, |this, cx| {
                                        this.radio_selected = i;
                                        cx.notify();
                                    })
                                    .ok();
                                }),
                        );
                    }
                    col.child(Radio::new("r-dis", false).label("Disabled").disabled(true))
                },
                colors,
            )
            .into_any_element(),
            // -------- Slider --------
            card(
                "Slider",
                v_flex()
                    .gap(Spacing::Small.pixels())
                    .child({
                        let w = weak.clone();
                        Slider::new("sl-basic", self.slider_basic)
                            .label("Volume")
                            .show_value(true)
                            .on_change(move |val, _, cx| {
                                w.update(cx, |this, cx| {
                                    this.slider_basic = val;
                                    cx.notify();
                                })
                                .ok();
                            })
                    })
                    .child({
                        let w = weak.clone();
                        Slider::new("sl-stepped", self.slider_stepped)
                            .min(0.0)
                            .max(100.0)
                            .step(10.0)
                            .label("Brightness (step 10)")
                            .show_value(true)
                            .on_change(move |val, _, cx| {
                                w.update(cx, |this, cx| {
                                    this.slider_stepped = val;
                                    cx.notify();
                                })
                                .ok();
                            })
                    })
                    .child(
                        Slider::new("sl-dis", 60.0)
                            .label("Disabled")
                            .show_value(true)
                            .disabled(true),
                    ),
                colors,
            )
            .into_any_element(),
            // -------- Stepper --------
            card(
                "Stepper",
                v_flex()
                    .gap(Spacing::Small.pixels())
                    .child({
                        let w = weak.clone();
                        Stepper::new("stp-qty", self.stepper_value as f64)
                            .label("Quantity")
                            .min(0.0)
                            .max(20.0)
                            .on_change(move |val, _, cx| {
                                w.update(cx, |this, cx| {
                                    this.stepper_value = val as f32;
                                    cx.notify();
                                })
                                .ok();
                            })
                    })
                    .child(Stepper::new("stp-min", 0.0).min(0.0).max(10.0))
                    .child(Stepper::new("stp-dis", 5.0).disabled(true)),
                colors,
            )
            .into_any_element(),
            // -------- TextField --------
            card(
                "TextField (Enter to submit)",
                v_flex()
                    .gap(Spacing::Small.pixels())
                    .child(self.text_field.clone())
                    .child(
                        Label::new(if self.submitted_value.is_empty() {
                            SharedString::from("Last submitted: (none yet)")
                        } else {
                            format!("Last submitted: {}", self.submitted_value).into()
                        })
                        .size(TextSize::Small)
                        .color(Color::Muted),
                    ),
                colors,
            )
            .into_any_element(),
            // -------- Chip / CountBadge --------
            card(
                "Chip & CountBadge",
                v_flex()
                    .gap(Spacing::Small.pixels())
                    .child(
                        h_flex()
                            .gap(Spacing::XSmall.pixels())
                            .flex_wrap()
                            .child(Chip::new("Default"))
                            .child(Chip::new("Accent").style(ChipStyle::Accent))
                            .child(Chip::new("Success").style(ChipStyle::Success))
                            .child(Chip::new("Warning").style(ChipStyle::Warning))
                            .child(Chip::new("Error").style(ChipStyle::Error)),
                    )
                    .child(
                        h_flex()
                            .gap(Spacing::Small.pixels())
                            .items_center()
                            .child(CountBadge::new(3))
                            .child(CountBadge::new(42))
                            .child(CountBadge::new(150)),
                    ),
                colors,
            )
            .into_any_element(),
            // -------- Indicator --------
            card(
                "Indicator",
                h_flex()
                    .gap(Spacing::Medium.pixels())
                    .items_center()
                    .child(Indicator::dot())
                    .child(Indicator::dot().color(Color::Success))
                    .child(Indicator::dot().color(Color::Warning))
                    .child(Indicator::dot().color(Color::Error))
                    .child(
                        v_flex()
                            .w(px(40.0))
                            .child(Indicator::bar().color(Color::Accent)),
                    )
                    .child(Indicator::icon(Icon::new(IconName::Check)).color(Color::Success))
                    .child(Indicator::icon(Icon::new(IconName::Close)).color(Color::Error)),
                colors,
            )
            .into_any_element(),
            // -------- Avatar / Facepile --------
            card(
                "Avatar & Facepile",
                v_flex()
                    .gap(Spacing::Small.pixels())
                    .child(
                        h_flex()
                            .gap(Spacing::Small.pixels())
                            .items_center()
                            .child(Avatar::new("Ada").size(AvatarSize::Small))
                            .child(Avatar::new("Linus"))
                            .child(Avatar::new("Grace").size(AvatarSize::Large))
                            .child(
                                Avatar::new("Ada")
                                    .size(AvatarSize::Small)
                                    .image(self.demo_image.clone()),
                            )
                            .child(Avatar::new("Linus").image(self.demo_image.clone()))
                            .child(
                                Avatar::new("Grace")
                                    .size(AvatarSize::Large)
                                    .image(self.demo_image.clone()),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap(Spacing::Medium.pixels())
                            .items_center()
                            .child(
                                Facepile::new()
                                    .push(Avatar::new("Ada"))
                                    .push(Avatar::new("Linus"))
                                    .push(Avatar::new("Grace"))
                                    .push(Avatar::new("Donald")),
                            )
                            .child(
                                Facepile::new()
                                    .push(Avatar::new("Ada").image(self.demo_image.clone()))
                                    .push(Avatar::new("Linus").image(self.demo_image.clone()))
                                    .push(Avatar::new("Grace").image(self.demo_image.clone())),
                            ),
                    ),
                colors,
            )
            .into_any_element(),
            // -------- Image --------
            card(
                "Image",
                v_flex()
                    .gap(Spacing::Small.pixels())
                    .child(
                        h_flex()
                            .gap(Spacing::Small.pixels())
                            .items_center()
                            .child(gpui::img(demo_image_path()).w(px(120.0)).h(px(68.0)))
                            .child(
                                gpui::img(demo_image_path())
                                    .w(px(120.0))
                                    .h(px(68.0))
                                    .rounded(Radius::Small.pixels()),
                            )
                            .child(gpui::img(demo_image_path()).size(px(56.0)).rounded_full()),
                    )
                    .child({
                        use gpui::StyledImage as _;
                        gpui::img(demo_image_path())
                            .grayscale(true)
                            .w(px(280.0))
                            .h(px(72.0))
                            .rounded(Radius::XSmall.pixels())
                    }),
                colors,
            )
            .into_any_element(),
            // -------- Squircle --------
            card(
                "Squircle",
                h_flex()
                    .gap(Spacing::Small.pixels())
                    .items_center()
                    .flex_wrap()
                    .child(Squircle::new().size(px(48.0)).bordered(true))
                    .child(
                        Squircle::new()
                            .size(px(48.0))
                            .fill(SquircleFill::Surface)
                            .bordered(true),
                    )
                    .child(Squircle::new().size(px(48.0)).fill(SquircleFill::Muted))
                    .child(Squircle::new().size(px(48.0)).fill(SquircleFill::Accent))
                    .child(
                        Squircle::new()
                            .size(px(56.0))
                            .fill(SquircleFill::Muted)
                            .child(Icon::new(IconName::Check).color(Color::Success)),
                    )
                    .child(
                        Squircle::new()
                            .size(px(56.0))
                            .fill(SquircleFill::Muted)
                            .child(Label::new("Aa")),
                    )
                    .child(
                        Squircle::new()
                            .fill(SquircleFill::Transparent)
                            .bordered(true)
                            .width(px(72.0))
                            .height(px(48.0)),
                    ),
                colors,
            )
            .into_any_element(),
            // -------- Disclosure --------
            card(
                "Disclosure",
                v_flex()
                    .gap(Spacing::XSmall.pixels())
                    .child(
                        h_flex()
                            .gap(Spacing::XSmall.pixels())
                            .items_center()
                            .child(
                                Disclosure::new("disc-1", self.disclosure_open)
                                    .on_click(bool_toggle(&weak, |this| &mut this.disclosure_open)),
                            )
                            .child(Label::new("Advanced settings")),
                    )
                    .when(self.disclosure_open, |this| {
                        this.child(
                            v_flex()
                                .pl(px(24.0))
                                .gap(Spacing::XSmall.pixels())
                                .child(Label::new("Setting one").color(Color::Muted))
                                .child(Label::new("Setting two").color(Color::Muted))
                                .child(Label::new("Setting three").color(Color::Muted)),
                        )
                    })
                    .child(
                        h_flex()
                            .gap(Spacing::XSmall.pixels())
                            .items_center()
                            .child(Disclosure::new("disc-dis", false).disabled(true))
                            .child(Label::new("Disabled").color(Color::Disabled)),
                    ),
                colors,
            )
            .into_any_element(),
            // -------- Accordion --------
            card(
                "Accordion",
                {
                    let sections = ["Getting started", "Configuration", "FAQ"];
                    let bodies = [
                        "Install the package and call init() during startup.",
                        "Set theme, colors, and spacing tokens.",
                        "Yes, it works on Wayland.",
                    ];
                    let mut acc = Accordion::new();
                    for i in 0..3 {
                        let w = weak.clone();
                        acc = acc.child(
                            AccordionItem::new(
                                SharedString::from(format!("acc-{i}")),
                                sections[i],
                                self.accordion_expanded[i],
                            )
                            .body(Label::new(bodies[i]).color(Color::Muted))
                            .on_toggle(move |_, _, cx| {
                                w.update(cx, |this, cx| {
                                    this.accordion_expanded[i] = !this.accordion_expanded[i];
                                    cx.notify();
                                })
                                .ok();
                            }),
                        );
                    }
                    acc
                },
                colors,
            )
            .into_any_element(),
            // -------- List --------
            card(
                "List (click to select)",
                List::new()
                    .header("Navigation")
                    .child(self.nav_item("nav-home", IconName::ChevronRight, "Home", &weak))
                    .child(self.nav_item("nav-search", IconName::MagnifyingGlass, "Search", &weak))
                    .child(self.nav_item("nav-settings", IconName::Settings, "Settings", &weak))
                    .child(
                        ListItem::new("nav-dis")
                            .start_slot(Icon::new(IconName::Close))
                            .child(Label::new("Unavailable"))
                            .disabled(true),
                    ),
                colors,
            )
            .into_any_element(),
            // -------- Tree list --------
            card(
                "Tree list",
                List::new()
                    .header("Project")
                    .child(
                        ListItem::new("tree-src")
                            .spacing(ListItemSpacing::Dense)
                            .inset(true)
                            .indent_level(0)
                            .start_slot(Icon::new(IconName::Folder))
                            .child(Label::new("src")),
                    )
                    .child(
                        ListItem::new("tree-src-main")
                            .spacing(ListItemSpacing::Dense)
                            .inset(true)
                            .indent_level(1)
                            .start_slot(Icon::new(IconName::File))
                            .child(Label::new("main.rs")),
                    )
                    .child(
                        ListItem::new("tree-src-lib")
                            .spacing(ListItemSpacing::Dense)
                            .inset(true)
                            .indent_level(1)
                            .start_slot(Icon::new(IconName::Folder))
                            .child(Label::new("lib")),
                    )
                    .child(
                        ListItem::new("tree-src-lib-mod")
                            .spacing(ListItemSpacing::Dense)
                            .inset(true)
                            .indent_level(2)
                            .start_slot(Icon::new(IconName::File))
                            .child(Label::new("mod.rs")),
                    )
                    .child(
                        ListItem::new("tree-cargo")
                            .spacing(ListItemSpacing::Dense)
                            .inset(true)
                            .indent_level(0)
                            .start_slot(Icon::new(IconName::File))
                            .child(Label::new("Cargo.toml")),
                    ),
                colors,
            )
            .into_any_element(),
            // -------- TabBar --------
            card(
                "TabBar",
                v_flex()
                    .gap(Spacing::Small.pixels())
                    .child(
                        TabBar::new()
                            .child(self.tab("tab-overview", IconName::Home, "Overview", 0, &weak))
                            .child(self.tab("tab-files", IconName::Folder, "Files", 1, &weak))
                            .child(self.tab(
                                "tab-settings",
                                IconName::Settings,
                                "Settings",
                                2,
                                &weak,
                            )),
                    )
                    .child(
                        Label::new(match self.selected_tab {
                            0 => "Overview content",
                            1 => "Files content",
                            _ => "Settings content",
                        })
                        .color(Color::Muted),
                    ),
                colors,
            )
            .into_any_element(),
            // -------- Breadcrumb --------
            card(
                "Breadcrumb",
                Breadcrumb::new()
                    .child(
                        BreadcrumbItem::new("bc-home", "Home")
                            .icon(IconName::Home)
                            .on_click(|_, _, _| {}),
                    )
                    .child(
                        BreadcrumbItem::new("bc-proj", "Projects")
                            .icon(IconName::Folder)
                            .on_click(|_, _, _| {}),
                    )
                    .child(BreadcrumbItem::new("bc-cur", "gpui-shell").current(true)),
                colors,
            )
            .into_any_element(),
            // -------- Pagination --------
            card(
                "Pagination",
                {
                    let w = weak.clone();
                    Pagination::new("pg-main")
                        .current_page(self.pagination_page as usize)
                        .total_pages(20)
                        .on_click(move |page, _, cx| {
                            w.update(cx, |this, cx| {
                                this.pagination_page = page as u32;
                                cx.notify();
                            })
                            .ok();
                        })
                },
                colors,
            )
            .into_any_element(),
            // -------- KeyBinding / KeybindingHint --------
            card(
                "KeyBinding & KeybindingHint",
                v_flex()
                    .gap(Spacing::Small.pixels())
                    .child(
                        h_flex()
                            .gap(Spacing::Small.pixels())
                            .items_center()
                            .child(KeyBinding::new(["Cmd", "S"]))
                            .child(KeyBinding::new(["Ctrl", "Shift", "P"]))
                            .child(KeyBinding::new(["Esc"])),
                    )
                    .child(KeybindingHint::with_prefix(
                        "Press",
                        KeyBinding::new(["Enter"]),
                    ))
                    .child(
                        KeybindingHint::with_prefix("Hit", KeyBinding::new(["Esc"]))
                            .suffix("to dismiss"),
                    ),
                colors,
            )
            .into_any_element(),
            // -------- Banner --------
            card(
                "Banner",
                v_flex()
                    .gap(Spacing::XSmall.pixels())
                    .when(self.banner_visible, |this| {
                        this.child(
                            Banner::new(Severity::Info, "New version available")
                                .description("Engram 0.2 is ready to install.")
                                .action(
                                    Button::new("banner-update", "Update")
                                        .style(ButtonStyle::Tinted(TintColor::Accent))
                                        .size(ButtonSize::Compact),
                                )
                                .on_dismiss({
                                    let weak = weak.clone();
                                    move |_, _, cx| {
                                        weak.update(cx, |this, cx| {
                                            this.banner_visible = false;
                                            cx.notify();
                                        })
                                        .ok();
                                    }
                                }),
                        )
                    })
                    .when(!self.banner_visible, |this| {
                        this.child(
                            Button::new("banner-restore", "Restore banner")
                                .style(ButtonStyle::Subtle)
                                .size(ButtonSize::Compact)
                                .on_click(set_field(&weak, true, |this| &mut this.banner_visible)),
                        )
                    })
                    .child(Banner::new(Severity::Success, "All checks passed"))
                    .child(
                        Banner::new(Severity::Warning, "Disk usage at 90%")
                            .description("Consider freeing up some space."),
                    )
                    .child(Banner::new(Severity::Error, "Build failed")),
                colors,
            )
            .into_any_element(),
            // -------- Callout --------
            card(
                "Callout",
                v_flex()
                    .gap(Spacing::XSmall.pixels())
                    .child(
                        Callout::new()
                            .severity(Severity::Info)
                            .title("Heads up")
                            .description("This is an informational message."),
                    )
                    .child(
                        Callout::new()
                            .severity(Severity::Warning)
                            .title("Caution")
                            .description("Review settings before continuing."),
                    ),
                colors,
            )
            .into_any_element(),
            // -------- Notification --------
            card(
                "Notification",
                v_flex()
                    .gap(Spacing::XSmall.pixels())
                    .child(
                        Notification::new(Severity::Success, "Saved")
                            .description("Your changes were saved automatically."),
                    )
                    .child(
                        Notification::new(Severity::Error, "Sync failed")
                            .description("Check your network connection."),
                    ),
                colors,
            )
            .into_any_element(),
            // -------- ProgressBar --------
            card(
                "ProgressBar",
                v_flex()
                    .gap(Spacing::XSmall.pixels())
                    .child(ProgressBar::new(0.0, 100.0))
                    .child(ProgressBar::new(self.progress_value, 100.0))
                    .child(ProgressBar::new(90.0, 100.0))
                    .child(ProgressBar::new(100.0, 100.0)),
                colors,
            )
            .into_any_element(),
            // The continuously animating cards live in `animated_cards` and
            // are only built when the toggle is on - see the module docs.
            // -------- DescriptionList --------
            card(
                "DescriptionList",
                DescriptionList::new()
                    .bordered(true)
                    .label_width(90.0)
                    .entry("Status", Chip::new("Active").style(ChipStyle::Success))
                    .entry("Plan", Label::new("Pro"))
                    .entry("Created", Label::new("2026-01-15"))
                    .entry("Email", Label::new("alice@ex.com")),
                colors,
            )
            .into_any_element(),
            // -------- HighlightedLabel --------
            card(
                "HighlightedLabel",
                v_flex()
                    .gap(Spacing::XSmall.pixels())
                    .child(HighlightedLabel::new("hello world", vec![0, 1, 2, 3, 4]))
                    .child(HighlightedLabel::new("components.rs", vec![0, 4, 8, 10]))
                    .child(HighlightedLabel::from_ranges(
                        "find the needle in the haystack",
                        &[9..15, 20..23],
                    )),
                colors,
            )
            .into_any_element(),
            // -------- CopyButton --------
            card(
                "CopyButton",
                v_flex()
                    .gap(Spacing::XSmall.pixels())
                    .child(
                        h_flex()
                            .gap(Spacing::Small.pixels())
                            .items_center()
                            .child(Label::new("some-api-key-1234").size(TextSize::Small))
                            .child(CopyButton::new("copy-key", "some-api-key-1234")),
                    )
                    .child(
                        h_flex()
                            .gap(Spacing::Small.pixels())
                            .items_center()
                            .child(Label::new("secret value").size(TextSize::Small))
                            .child(
                                CopyButton::new("copy-sec", "secret").tooltip_label("Copy secret"),
                            ),
                    ),
                colors,
            )
            .into_any_element(),
            // -------- SplitButton --------
            card(
                "SplitButton",
                v_flex()
                    .gap(Spacing::XSmall.pixels())
                    .child(SplitButton::new(
                        IconButton::new("sb-play-l", IconName::Play),
                        IconButton::new("sb-play-r", IconName::ChevronDown),
                    ))
                    .child(
                        SplitButton::new(
                            IconButton::new("sb-save-l", IconName::Save),
                            IconButton::new("sb-save-r", IconName::ChevronDown),
                        )
                        .style(SplitButtonStyle::Outlined),
                    )
                    .child(
                        SplitButton::new(
                            IconButton::new("sb-set-l", IconName::Settings),
                            IconButton::new("sb-set-r", IconName::ChevronDown),
                        )
                        .style(SplitButtonStyle::Transparent),
                    ),
                colors,
            )
            .into_any_element(),
            // -------- Menu (anchored popover) --------
            card(
                "Menu",
                {
                    let bounds_slot = self.menu_trigger_bounds.clone();
                    let menu_entity = self.menu.clone();
                    let open_menu_handler = {
                        let weak = weak.clone();
                        move |_event: &gpui::ClickEvent, window: &mut Window, cx: &mut App| {
                            weak.update(cx, |this, cx| {
                                this.menu_open = !this.menu_open;
                                if this.menu_open {
                                    let handle = this.menu.read(cx).focus_handle().clone();
                                    window.focus(&handle, cx);
                                }
                                cx.notify();
                            })
                            .ok();
                        }
                    };
                    let trigger_button = Button::new("btn-menu-trigger", "Open menu")
                        .icon(IconName::ChevronDown)
                        .style(ButtonStyle::Outlined)
                        .on_click(open_menu_handler);
                    let trigger_with_capture = div().relative().child(trigger_button).child(
                        canvas(
                            move |bounds, _window, _cx| {
                                bounds_slot.set(Some(bounds));
                            },
                            |_, _, _, _| {},
                        )
                        .absolute()
                        .inset_0()
                        .size_full(),
                    );

                    let menu_open = self.menu_open;
                    let trigger_bounds = self.menu_trigger_bounds.get();
                    let anchor_focus = self.menu.read(cx).focus_handle().clone();
                    let weak_for_dismiss = weak.clone();

                    v_flex()
                        .gap(Spacing::Small.pixels())
                        .child(trigger_with_capture)
                        .when(menu_open, |this| {
                            let Some(bounds) = trigger_bounds else {
                                return this;
                            };
                            this.child(anchored_popover(
                                anchor_focus,
                                gpui::Anchor::TopLeft,
                                bounds,
                                menu_entity,
                                move |_window, cx| {
                                    weak_for_dismiss
                                        .update(cx, |this, cx| {
                                            this.menu_open = false;
                                            cx.notify();
                                        })
                                        .ok();
                                },
                            ))
                        })
                },
                colors,
            )
            .into_any_element(),
            // -------- Modal trigger --------
            card(
                "Modal",
                Button::new("btn-open-modal", "Open modal")
                    .style(ButtonStyle::Tinted(TintColor::Accent))
                    .on_click({
                        let weak = weak.clone();
                        move |_event, window, cx| {
                            weak.update(cx, |this, cx| {
                                this.modal_open = true;
                                window.focus(&this.modal_focus, cx);
                                cx.notify();
                            })
                            .ok();
                        }
                    }),
                colors,
            )
            .into_any_element(),
        ];

        let cards = {
            let mut cards = cards;
            if self.animations_on {
                cards.extend(animated_cards(colors));
            }
            cards
        };

        const COLS: usize = 5;
        let mut columns: Vec<Vec<gpui::AnyElement>> = (0..COLS).map(|_| Vec::new()).collect();
        for (i, card) in cards.into_iter().enumerate() {
            columns[i % COLS].push(card);
        }

        let grid = h_flex()
            .items_start()
            .gap(Spacing::Medium.pixels())
            .px(Spacing::Large.pixels())
            .pb(Spacing::Large.pixels())
            .children(
                columns
                    .into_iter()
                    .map(|col| v_flex().gap(Spacing::Medium.pixels()).children(col)),
            );

        v_flex()
            .id("showcase-root")
            .size_full()
            .font_family("Funnel Sans")
            .bg(colors.background)
            .overflow_y_scroll()
            .child(header)
            .child(grid)
            .when(self.modal_open, |this| {
                let weak_for_modal = weak.clone();
                let weak_for_buttons = weak.clone();
                this.child(modal_overlay(
                    self.modal_focus.clone(),
                    Modal::new()
                        .title("Delete file?")
                        .child(Label::new("This action cannot be undone.").color(Color::Muted))
                        .footer(
                            h_flex()
                                .gap(Spacing::Small.pixels())
                                .justify_end()
                                .child(
                                    Button::new("modal-cancel", "Cancel")
                                        .style(ButtonStyle::Subtle)
                                        .on_click(set_field(&weak_for_buttons, false, |this| {
                                            &mut this.modal_open
                                        })),
                                )
                                .child(
                                    Button::new("modal-delete", "Delete")
                                        .style(ButtonStyle::Tinted(TintColor::Accent))
                                        .on_click(set_field(&weak_for_buttons, false, |this| {
                                            &mut this.modal_open
                                        })),
                                ),
                        ),
                    move |_window, cx| {
                        weak_for_modal
                            .update(cx, |this, cx| {
                                this.modal_open = false;
                                cx.notify();
                            })
                            .ok();
                    },
                ))
            })
    }
}

impl Showcase {
    fn nav_item(
        &self,
        id: &'static str,
        icon: IconName,
        label: &'static str,
        weak: &WeakEntity<Self>,
    ) -> ListItem {
        let id_owned = SharedString::from(id);
        let is_selected = self.selected_nav == id_owned;
        let weak = weak.clone();
        ListItem::new(id)
            .start_slot(Icon::new(icon))
            .child(Label::new(label))
            .toggle_state(is_selected)
            .on_click(move |_event, _window, cx| {
                let id = id_owned.clone();
                weak.update(cx, |this, cx| {
                    this.selected_nav = id;
                    cx.notify();
                })
                .ok();
            })
    }

    fn tab(
        &self,
        id: &'static str,
        icon: IconName,
        label: &'static str,
        index: usize,
        weak: &WeakEntity<Self>,
    ) -> Tab {
        let weak = weak.clone();
        Tab::new(id, label)
            .icon(icon)
            .toggle_state(self.selected_tab == index)
            .on_click(move |_event, _window, cx| {
                weak.update(cx, |this, cx| {
                    this.selected_tab = index;
                    cx.notify();
                })
                .ok();
            })
    }
}

/// The cards that animate continuously, and therefore drive a repaint every
/// frame while they are on screen. Built only when the toggle is on.
/// Filesystem path to the demo image.
///
/// Built as a `PathBuf` on purpose: `img()` reads a `&str` as an asset
/// resource to look up in the registered `AssetSource`, so passing the path
/// as a string silently renders nothing.
fn demo_image_path() -> std::path::PathBuf {
    std::path::PathBuf::from(DEMO_IMAGE)
}

fn animated_cards(colors: ThemeColors) -> Vec<gpui::AnyElement> {
    vec![
        // -------- CircularProgress + Spinner --------
        card(
            "CircularProgress & Spinner",
            v_flex()
                .gap(Spacing::Small.pixels())
                .child(
                    h_flex()
                        .gap(Spacing::Medium.pixels())
                        .items_center()
                        .child(CircularProgress::new(0.25, 1.0, px(28.0)))
                        .child(CircularProgress::new(0.5, 1.0, px(28.0)))
                        .child(CircularProgress::new(0.75, 1.0, px(28.0)))
                        .child(CircularProgress::new(1.0, 1.0, px(28.0)))
                        .child(CircularProgress::new(0.6, 1.0, px(40.0))),
                )
                .child(
                    h_flex()
                        .gap(Spacing::Medium.pixels())
                        .items_center()
                        .child(Spinner::new().size(IconSize::XSmall))
                        .child(Spinner::new().size(IconSize::Small))
                        .child(Spinner::new())
                        .child(Spinner::new().size(IconSize::XLarge))
                        .child(Spinner::new().color(Color::Accent))
                        .child(Label::new("Loading...").color(Color::Muted)),
                ),
            colors,
        )
        .into_any_element(),
        // -------- Skeleton --------
        card(
            "Skeleton",
            h_flex()
                .gap(Spacing::Medium.pixels())
                .items_start()
                .child(
                    h_flex()
                        .gap(Spacing::Small.pixels())
                        .items_center()
                        .child(Skeleton::circle(px(36.0)))
                        .child(
                            v_flex()
                                .gap(px(4.0))
                                .child(Skeleton::new().width(px(100.0)).height(px(12.0)))
                                .child(Skeleton::new().width(px(60.0)).height(px(10.0))),
                        ),
                )
                .child(skeleton_text(3, px(160.0))),
            colors,
        )
        .into_any_element(),
    ]
}

fn card(title: &'static str, body: impl IntoElement, colors: ThemeColors) -> impl IntoElement {
    v_flex()
        .p(Spacing::Medium.pixels())
        .gap(Spacing::Small.pixels())
        .rounded(Radius::Small.pixels())
        .bg(colors.elevated_surface_background)
        .border_1()
        .border_color(colors.border_variant)
        .child(Label::new(title).size(TextSize::Small).color(Color::Muted))
        .child(body)
}

fn main() {
    application()
        .with_assets(assets::Assets)
        .run(|cx: &mut App| {
            Theme::init(cx);
            ui::init(cx);

            let bounds = Bounds::centered(None, size(px(1440.0), px(960.0)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |window, cx| {
                    window.set_rem_size(cx.theme().font_size);
                    cx.new(Showcase::new)
                },
            )
            .unwrap();

            cx.activate(true);
        });
}

//! [`ToggleButtonGroup`] - a segmented control where exactly one button in
//! the group is selected at a time.

use std::rc::Rc;

use gpui::{
    AnyElement, AnyView, App, ClickEvent, DefiniteLength, IntoElement, RenderOnce, SharedString,
    Styled, Window, div, prelude::*, relative,
};

use crate::theme::{ActiveTheme, Color};

use super::button_like::{
    ButtonCommon, ButtonLike, ButtonLikeRounding, ButtonSize, ButtonStyle, SelectableButton,
    TintColor,
};
use crate::components::icon::{Icon, IconName, IconSize};
use crate::components::label::{Label, LabelCommon};
use crate::components::stack::{h_flex, v_flex};
use crate::theme::TextSize;
use crate::traits::handlers::{ClickHandler, TooltipBuilder};
use crate::traits::{Clickable, Toggleable};

/// The position of a button within the group, determining which corners get rounding.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
struct ToggleButtonPosition {
    leftmost: bool,
    rightmost: bool,
}

impl ToggleButtonPosition {
    fn to_rounding(self) -> ButtonLikeRounding {
        ButtonLikeRounding {
            top_left: self.leftmost,
            top_right: self.rightmost,
            bottom_right: self.rightmost,
            bottom_left: self.leftmost,
        }
    }
}

/// Configuration extracted from a button builder before rendering.
#[doc(hidden)]
pub struct ButtonConfiguration {
    label: SharedString,
    icon: Option<IconName>,
    on_click: ClickHandler,
    selected: bool,
    tooltip: Option<TooltipBuilder>,
}

/// Trait implemented by toggle button builder types.
pub trait ButtonBuilder: 'static + sealed::Sealed {
    #[doc(hidden)]
    fn into_configuration(self) -> ButtonConfiguration;
}

mod sealed {
    pub trait Sealed {}
    impl Sealed for super::ToggleButtonSimple {}
    impl Sealed for super::ToggleButtonWithIcon {}
}

/// A simple toggle button with just a label.
pub struct ToggleButtonSimple {
    label: SharedString,
    on_click: ClickHandler,
    selected: bool,
    tooltip: Option<TooltipBuilder>,
}

impl ToggleButtonSimple {
    pub fn new(
        label: impl Into<SharedString>,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            label: label.into(),
            on_click: Rc::new(on_click),
            selected: false,
            tooltip: None,
        }
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn tooltip(mut self, tooltip: impl Fn(&mut Window, &mut App) -> AnyView + 'static) -> Self {
        self.tooltip = Some(Rc::new(tooltip));
        self
    }
}

impl ButtonBuilder for ToggleButtonSimple {
    fn into_configuration(self) -> ButtonConfiguration {
        ButtonConfiguration {
            label: self.label,
            icon: None,
            on_click: self.on_click,
            selected: self.selected,
            tooltip: self.tooltip,
        }
    }
}

/// A toggle button with a label and an icon.
pub struct ToggleButtonWithIcon {
    label: SharedString,
    icon: IconName,
    on_click: ClickHandler,
    selected: bool,
    tooltip: Option<TooltipBuilder>,
}

impl ToggleButtonWithIcon {
    pub fn new(
        label: impl Into<SharedString>,
        icon: IconName,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            label: label.into(),
            icon,
            on_click: Rc::new(on_click),
            selected: false,
            tooltip: None,
        }
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn tooltip(mut self, tooltip: impl Fn(&mut Window, &mut App) -> AnyView + 'static) -> Self {
        self.tooltip = Some(Rc::new(tooltip));
        self
    }
}

impl ButtonBuilder for ToggleButtonWithIcon {
    fn into_configuration(self) -> ButtonConfiguration {
        ButtonConfiguration {
            label: self.label,
            icon: Some(self.icon),
            on_click: self.on_click,
            selected: self.selected,
            tooltip: self.tooltip,
        }
    }
}

/// Visual style of the toggle button group container.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToggleButtonGroupStyle {
    Transparent,
    Filled,
    Outlined,
}

/// A group of mutually exclusive toggle buttons, visually joined as a
/// segmented control.
#[derive(IntoElement)]
#[must_use = "ToggleButtonGroup does nothing unless rendered"]
pub struct ToggleButtonGroup<T: ButtonBuilder, const N: usize> {
    group_name: SharedString,
    buttons: [T; N],
    style: ToggleButtonGroupStyle,
    size: ButtonSize,
    label_size: TextSize,
    group_width: Option<DefiniteLength>,
    auto_width: bool,
    selected_index: usize,
}

impl<T: ButtonBuilder, const N: usize> ToggleButtonGroup<T, N> {
    pub fn new(group_name: impl Into<SharedString>, buttons: [T; N]) -> Self {
        Self {
            group_name: group_name.into(),
            buttons,
            style: ToggleButtonGroupStyle::Transparent,
            size: ButtonSize::Default,
            label_size: TextSize::Small,
            group_width: None,
            auto_width: false,
            selected_index: 0,
        }
    }

    pub fn style(mut self, style: ToggleButtonGroupStyle) -> Self {
        self.style = style;
        self
    }

    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    pub fn selected_index(mut self, index: usize) -> Self {
        self.selected_index = index;
        self
    }

    pub fn auto_width(mut self) -> Self {
        self.auto_width = true;
        self
    }

    pub fn label_size(mut self, label_size: TextSize) -> Self {
        self.label_size = label_size;
        self
    }

    pub fn width(mut self, width: impl Into<DefiniteLength>) -> Self {
        self.group_width = Some(width.into());
        self
    }

    pub fn full_width(mut self) -> Self {
        self.group_width = Some(relative(1.));
        self
    }
}

impl<T: ButtonBuilder, const N: usize> RenderOnce for ToggleButtonGroup<T, N> {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors();
        let border_color = colors.border.opacity(0.6);
        let is_outlined = self.style == ToggleButtonGroupStyle::Outlined;
        let is_filled = self.style == ToggleButtonGroupStyle::Filled;
        let is_transparent = self.style == ToggleButtonGroupStyle::Transparent;
        // Inner buttons are transparent in every group style - the group's
        // container paints the chrome (background, border).
        let inner_style = ButtonStyle::Transparent;

        let entries: Vec<AnyElement> = self
            .buttons
            .into_iter()
            .enumerate()
            .map(|(i, button)| {
                let ButtonConfiguration {
                    label,
                    icon,
                    on_click,
                    selected,
                    tooltip,
                } = button.into_configuration();

                let is_selected = i == self.selected_index || selected;

                let btn = ButtonLike::new((self.group_name.clone(), i))
                    .when(!self.auto_width, |this| this.full_width())
                    .rounding(Some(
                        ToggleButtonPosition {
                            leftmost: i == 0,
                            rightmost: i == N - 1,
                        }
                        .to_rounding(),
                    ))
                    .style(inner_style)
                    .when(is_selected, |this| {
                        this.toggle_state(true)
                            .selected_style(ButtonStyle::Tinted(TintColor::Accent))
                    })
                    .size(self.size)
                    .child(
                        h_flex()
                            .w_full()
                            .px_2()
                            .gap_1p5()
                            .justify_center()
                            .flex_none()
                            .when_some(icon, |this, icon_name| {
                                this.py_2().child(
                                    Icon::new(icon_name).size(IconSize::XSmall).color(
                                        if is_selected {
                                            Color::Accent
                                        } else {
                                            Color::Muted
                                        },
                                    ),
                                )
                            })
                            .child(
                                Label::new(label)
                                    .size(self.label_size)
                                    .when(is_selected, |this| this.color(Color::Accent)),
                            ),
                    )
                    .when_some(tooltip, |this: ButtonLike, tt| {
                        this.tooltip(move |window, cx| tt(window, cx))
                    })
                    .on_click(move |event, window, cx| on_click(event, window, cx));

                // Outlined shows a thin divider between cells; Filled keeps
                // them flush so the group reads as a single surface.
                let last_item = i == N - 1;
                div()
                    .when(is_outlined && !last_item, |this| {
                        this.border_r_1().border_color(border_color)
                    })
                    .when(!self.auto_width, |this| this.w(relative(1. / N as f32)))
                    .overflow_hidden()
                    .child(btn)
                    .into_any_element()
            })
            .collect();

        v_flex()
            .map(|this| {
                if let Some(w) = self.group_width {
                    this.w(w)
                } else if self.auto_width {
                    this
                } else {
                    this.w_full()
                }
            })
            .rounded_md()
            .overflow_hidden()
            .when(is_filled, |this| this.bg(colors.element_background))
            .when(is_outlined, |this| {
                this.border_1().border_color(border_color)
            })
            .child(
                h_flex()
                    .when(is_transparent || is_filled, |this| this.gap_px())
                    .children(entries),
            )
    }
}

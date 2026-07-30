//! [`ButtonLike`] - the shared chrome behind every button.

use std::rc::Rc;

use crate::theme::{ActiveTheme, Radius};
use gpui::{
    AnyElement, AnyView, App, ClickEvent, CursorStyle, DefiniteLength, Div, ElementId, FocusHandle,
    Hsla, IntoElement, MouseButton, ParentElement, Pixels, RenderOnce, StyleRefinement, Window,
    div, prelude::*, relative, transparent_black,
};
use smallvec::SmallVec;

use crate::traits::{
    ClickHandler, Clickable, Disableable, StyledExt, ToggleState, Toggleable, TooltipBuilder,
};

/// Buttons that can swap their [`ButtonStyle`] when in the selected state.
pub trait SelectableButton: Toggleable {
    fn selected_style(self, style: ButtonStyle) -> Self;
}

/// The "every button speaks the same dialect" trait - id, style, size,
/// tooltip, tab index, focus tracking.
pub trait ButtonCommon: Clickable + Disableable {
    /// The button's element id.
    fn id(&self) -> &ElementId;

    /// Set the visual style. Defaults to [`ButtonStyle::Filled`].
    fn style(self, style: ButtonStyle) -> Self;

    /// Set the size preset. Defaults to [`ButtonSize::Default`].
    fn size(self, size: ButtonSize) -> Self;

    /// Attach a tooltip builder.
    fn tooltip(self, tooltip: impl Fn(&mut Window, &mut App) -> AnyView + 'static) -> Self;

    /// Insert this button into the keyboard tab order at `tab_index`.
    fn tab_index(self, tab_index: isize) -> Self;

    /// Track focus on the given handle.
    fn track_focus(self, focus_handle: &FocusHandle) -> Self;
}

/// The visual variant of a button.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ButtonStyle {
    /// Solid filled background.
    #[default]
    Filled,
    /// A semantic-coloured tint (Accent / Error / Warning / Success) - soft
    /// background plus a coloured border.
    Tinted(TintColor),
    /// Bordered, transparent-until-hover background.
    Outlined,
    /// Like [`ButtonStyle::Outlined`] but with a more recessive (variant) border tone.
    OutlinedGhost,
    /// Transparent until hover. Toolbar / inline-action style.
    Subtle,
    /// Transparent with no border, tinting only on hover/active.
    Ghost,
    /// Fully transparent in every state.
    Transparent,
}

/// Tint flavor for [`ButtonStyle::Tinted`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum TintColor {
    #[default]
    Accent,
    Error,
    Warning,
    Success,
}

/// Resolved background + border colors for one (style x state) pair.
#[derive(Debug, Clone, Copy)]
pub(super) struct ButtonLikeStyles {
    pub background: Hsla,
    pub border: Hsla,
}

impl TintColor {
    /// The foreground status color for this tint flavor.
    fn foreground(self, cx: &App) -> Hsla {
        let status = &cx.theme().colors().status;
        match self {
            TintColor::Accent => status.info,
            TintColor::Error => status.error,
            TintColor::Warning => status.warning,
            TintColor::Success => status.success,
        }
    }

    fn enabled_styles(self, cx: &App) -> ButtonLikeStyles {
        let fg = self.foreground(cx);
        ButtonLikeStyles {
            background: fg.opacity(0.15),
            border: fg.opacity(0.55),
        }
    }

    fn hovered_styles(self, cx: &App) -> ButtonLikeStyles {
        let fg = self.foreground(cx);
        ButtonLikeStyles {
            background: fg.opacity(0.25),
            border: fg.opacity(0.60),
        }
    }
}

impl ButtonStyle {
    pub(super) fn enabled(self, cx: &App) -> ButtonLikeStyles {
        let colors = cx.theme().colors();
        match self {
            // Inverted "primary" - fg as background, bg as label.
            ButtonStyle::Filled => ButtonLikeStyles {
                background: colors.text,
                border: transparent_black(),
            },
            ButtonStyle::Tinted(tint) => tint.enabled_styles(cx),
            // Transparent fill with a strong border - clearly distinct from
            // `Subtle` (which has a filled surface).
            ButtonStyle::Outlined => ButtonLikeStyles {
                background: transparent_black(),
                border: colors.border_selected,
            },
            ButtonStyle::OutlinedGhost => ButtonLikeStyles {
                background: transparent_black(),
                border: colors.border_variant,
            },
            // Filled surface with a border - mirrors the draft's `.btn.subtle`.
            ButtonStyle::Subtle => ButtonLikeStyles {
                background: colors.element_background,
                border: colors.border,
            },
            ButtonStyle::Ghost | ButtonStyle::Transparent => ButtonLikeStyles {
                background: transparent_black(),
                border: transparent_black(),
            },
        }
    }

    pub(super) fn hovered(self, cx: &App) -> ButtonLikeStyles {
        let colors = cx.theme().colors();
        match self {
            ButtonStyle::Filled => ButtonLikeStyles {
                background: colors.text_muted,
                border: transparent_black(),
            },
            // Tinted backgrounds are alpha-blended from the status foreground
            // color; hover bumps the alpha to give feedback without an extra
            // darken pass.
            ButtonStyle::Tinted(tint) => tint.hovered_styles(cx),
            ButtonStyle::Outlined => ButtonLikeStyles {
                background: colors.ghost_element_hover,
                border: colors.border_selected,
            },
            ButtonStyle::OutlinedGhost => ButtonLikeStyles {
                background: colors.ghost_element_hover,
                border: colors.border_variant,
            },
            ButtonStyle::Subtle => ButtonLikeStyles {
                background: colors.element_hover,
                border: colors.border,
            },
            ButtonStyle::Ghost => ButtonLikeStyles {
                background: colors.ghost_element_hover,
                border: transparent_black(),
            },
            ButtonStyle::Transparent => ButtonLikeStyles {
                background: transparent_black(),
                border: transparent_black(),
            },
        }
    }

    pub(super) fn active(self, cx: &App) -> ButtonLikeStyles {
        let colors = cx.theme().colors();
        match self {
            ButtonStyle::Filled => ButtonLikeStyles {
                background: colors.text_placeholder,
                border: transparent_black(),
            },
            ButtonStyle::Tinted(tint) => {
                let fg = tint.foreground(cx);
                ButtonLikeStyles {
                    background: fg.opacity(0.32),
                    border: fg.opacity(0.65),
                }
            }
            ButtonStyle::Outlined => ButtonLikeStyles {
                background: colors.ghost_element_active,
                border: colors.border_selected,
            },
            ButtonStyle::OutlinedGhost => ButtonLikeStyles {
                background: transparent_black(),
                border: colors.border_variant,
            },
            ButtonStyle::Subtle => ButtonLikeStyles {
                background: colors.element_active,
                border: colors.border,
            },
            ButtonStyle::Ghost => ButtonLikeStyles {
                background: colors.ghost_element_active,
                border: transparent_black(),
            },
            ButtonStyle::Transparent => ButtonLikeStyles {
                background: transparent_black(),
                border: transparent_black(),
            },
        }
    }

    pub(super) fn disabled_styles(self, cx: &App) -> ButtonLikeStyles {
        let colors = cx.theme().colors();
        match self {
            ButtonStyle::Filled => ButtonLikeStyles {
                background: colors.element_disabled,
                border: transparent_black(),
            },
            ButtonStyle::Subtle | ButtonStyle::Outlined | ButtonStyle::Tinted(_) => {
                ButtonLikeStyles {
                    background: colors.element_disabled,
                    border: colors.border_disabled,
                }
            }
            ButtonStyle::OutlinedGhost => ButtonLikeStyles {
                background: transparent_black(),
                border: colors.border_disabled,
            },
            ButtonStyle::Ghost | ButtonStyle::Transparent => ButtonLikeStyles {
                background: transparent_black(),
                border: transparent_black(),
            },
        }
    }

    /// Whether this style ever paints a visible border.
    pub(super) fn is_outlined(self) -> bool {
        matches!(
            self,
            ButtonStyle::Outlined
                | ButtonStyle::OutlinedGhost
                | ButtonStyle::Subtle
                | ButtonStyle::Tinted(_)
        )
    }

    /// Override for the label / icon color when the button's chrome demands a
    /// non-default foreground (e.g. `Filled` is an inverted slab, so the
    /// label must flip to the app background to stay legible).
    pub(super) fn label_color_override(self, cx: &App) -> Option<crate::theme::Color> {
        match self {
            ButtonStyle::Filled => {
                Some(crate::theme::Color::Custom(cx.theme().colors().background))
            }
            _ => None,
        }
    }
}

/// Button height presets.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ButtonSize {
    Compact,
    #[default]
    Default,
    Large,
}

/// Per-corner rounding control for buttons in segmented groups.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) struct ButtonLikeRounding {
    pub top_left: bool,
    pub top_right: bool,
    pub bottom_right: bool,
    pub bottom_left: bool,
}

impl ButtonLikeRounding {
    pub(crate) const ALL: Self = Self {
        top_left: true,
        top_right: true,
        bottom_right: true,
        bottom_left: true,
    };
}

/// Shared chrome behind every button. See the module docs.
#[derive(IntoElement)]
#[must_use = "ButtonLike does nothing unless rendered"]
pub struct ButtonLike {
    pub(super) base: Div,
    pub(super) id: ElementId,
    pub(super) style: ButtonStyle,
    pub(super) size: ButtonSize,
    pub(super) disabled: bool,
    pub(super) selected: bool,
    pub(super) selected_style: Option<ButtonStyle>,
    pub(super) focus_handle: Option<FocusHandle>,
    pub(super) tab_index: Option<isize>,
    pub(super) cursor_style: CursorStyle,
    pub(super) tooltip: Option<TooltipBuilder>,
    pub(super) on_click: Option<ClickHandler>,
    pub(super) on_aux_click: Option<ClickHandler>,
    pub(super) children: SmallVec<[AnyElement; 2]>,
    pub(super) horizontal_padding: Option<Pixels>,
    pub(super) vertical_padding: Option<Pixels>,
    pub(super) rounding: Option<ButtonLikeRounding>,
    pub(super) width: Option<DefiniteLength>,
}

impl ButtonLike {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div(),
            id: id.into(),
            style: ButtonStyle::default(),
            size: ButtonSize::default(),
            disabled: false,
            selected: false,
            selected_style: None,
            focus_handle: None,
            tab_index: None,
            cursor_style: CursorStyle::PointingHand,
            tooltip: None,
            on_click: None,
            on_aux_click: None,
            children: SmallVec::new(),
            horizontal_padding: None,
            vertical_padding: None,
            rounding: Some(ButtonLikeRounding::ALL),
            width: None,
        }
    }

    /// Set per-corner rounding. `None` means no rounding at all.
    pub(crate) fn rounding(mut self, rounding: impl Into<Option<ButtonLikeRounding>>) -> Self {
        self.rounding = rounding.into();
        self
    }

    /// Handle the non-primary mouse buttons - right and middle - in one callback.
    pub fn on_aux_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_aux_click = Some(Rc::new(handler));
        self
    }

    /// Set a fixed width for this button.
    pub fn width(mut self, width: impl Into<DefiniteLength>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// Set the button to fill its parent width.
    pub fn full_width(mut self) -> Self {
        self.width = Some(relative(1.));
        self
    }

    /// Set the inner padding (horizontal, vertical) of this button.
    pub fn padding(mut self, horizontal: Pixels, vertical: Pixels) -> Self {
        self.horizontal_padding = Some(horizontal);
        self.vertical_padding = Some(vertical);
        self
    }
}

impl ParentElement for ButtonLike {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Disableable for ButtonLike {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Toggleable for ButtonLike {
    fn toggle_state(mut self, state: impl Into<ToggleState>) -> Self {
        self.selected = state.into().selected();
        self
    }
}

impl SelectableButton for ButtonLike {
    fn selected_style(mut self, style: ButtonStyle) -> Self {
        self.selected_style = Some(style);
        self
    }
}

impl Clickable for ButtonLike {
    fn on_click(mut self, handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    fn cursor_style(mut self, cursor_style: CursorStyle) -> Self {
        self.cursor_style = cursor_style;
        self
    }
}

impl ButtonCommon for ButtonLike {
    fn id(&self) -> &ElementId {
        &self.id
    }

    fn style(mut self, style: ButtonStyle) -> Self {
        self.style = style;
        self
    }

    fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    fn tooltip(mut self, tooltip: impl Fn(&mut Window, &mut App) -> AnyView + 'static) -> Self {
        self.tooltip = Some(Rc::new(tooltip));
        self
    }

    fn tab_index(mut self, tab_index: isize) -> Self {
        self.tab_index = Some(tab_index);
        self
    }

    fn track_focus(mut self, focus_handle: &FocusHandle) -> Self {
        self.focus_handle = Some(focus_handle.clone());
        self
    }
}

impl RenderOnce for ButtonLike {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        // When `selected` is true but no explicit `selected_style` was set,
        // fall back to a tinted-accent palette so selection is visible even
        // on low-contrast base styles (Subtle, Transparent).
        let style = if self.selected {
            self.selected_style
                .unwrap_or(ButtonStyle::Tinted(TintColor::Accent))
        } else {
            self.style
        };

        let enabled = style.enabled(cx);
        let hovered = style.hovered(cx);
        let active = style.active(cx);
        let disabled_palette = style.disabled_styles(cx);
        let is_outlined = style.is_outlined();
        let is_disabled = self.disabled;
        let cursor = self.cursor_style;

        let on_click = self.on_click;
        let on_aux_click = self.on_aux_click;
        let tooltip = self.tooltip;
        let focus_handle = self.focus_handle;
        let tab_index = self.tab_index;
        let children = self.children;
        let horizontal_padding = self.horizontal_padding;
        let vertical_padding = self.vertical_padding;

        self.base
            .id(self.id)
            .h_flex()
            .when_some(self.width, |this, w| this.w(w).justify_center())
            .map(|this| {
                let r = Radius::Small.pixels();
                match self.rounding {
                    Some(rounding) => this
                        .when(rounding.top_left, |e| e.rounded_tl(r))
                        .when(rounding.top_right, |e| e.rounded_tr(r))
                        .when(rounding.bottom_right, |e| e.rounded_br(r))
                        .when(rounding.bottom_left, |e| e.rounded_bl(r)),
                    None => this,
                }
            })
            .when_some(horizontal_padding, |this, p| this.px(p))
            .when_some(vertical_padding, |this, p| this.py(p))
            .when(is_outlined, |this| this.border_1())
            .map(|this| {
                if is_disabled {
                    let this = this.bg(disabled_palette.background);
                    if is_outlined {
                        this.border_color(disabled_palette.border)
                    } else {
                        this
                    }
                } else {
                    let this = this.bg(enabled.background);
                    let this = if is_outlined {
                        this.border_color(enabled.border)
                    } else {
                        this
                    };
                    this.cursor(cursor)
                        .hover(move |s: StyleRefinement| {
                            let s = s.bg(hovered.background);
                            if is_outlined {
                                s.border_color(hovered.border)
                            } else {
                                s
                            }
                        })
                        .active(move |s: StyleRefinement| {
                            let s = s.bg(active.background);
                            if is_outlined {
                                s.border_color(active.border)
                            } else {
                                s
                            }
                        })
                }
            })
            .when_some(tab_index, |this, ix| this.tab_index(ix))
            .when_some(focus_handle, |this, fh| this.track_focus(&fh))
            .when_some(tooltip, |this, builder| {
                this.tooltip(move |window, cx| builder(window, cx))
            })
            .when_some(on_click.filter(|_| !is_disabled), |this, handler| {
                this.on_click(move |event, window, cx| handler(event, window, cx))
            })
            .when_some(on_aux_click.filter(|_| !is_disabled), |this, handler| {
                this.on_mouse_down(MouseButton::Right, |_, window, _| window.prevent_default())
                    .on_aux_click(move |event, window, cx| {
                        cx.stop_propagation();
                        handler(event, window, cx)
                    })
            })
            .children(children)
    }
}

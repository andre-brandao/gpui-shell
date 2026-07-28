//! Avatar / Facepile / Chip / CountBadge - small data-display primitives.
//!
//! Grouped in one file because each is tiny and they share a similar shape.
//!
//! - [`Avatar`]: a circular profile bubble. Defaults to a monogram (initial
//!   letter over a hash-derived hue) but can load a real image via
//!   [`Avatar::image`].
//! - [`Facepile`]: a horizontal stack of overlapping `Avatar`s.
//! - [`Chip`]: a compact rounded badge for a single label, optionally
//!   colored by status (Default / Accent / Success / Warning / Error / Info).
//!   Supports size variants and an outline mode.
//! - [`CountBadge`]: a numeric badge that styles small counts ("3") and
//!   caps large ones at "99+".

use crate::theme::{ActiveTheme, Color, Radius, Spacing};
use gpui::{
    App, Hsla, ImageSource, IntoElement, ParentElement, Pixels, RenderOnce, SharedString, Styled,
    Window, div, hsla, img, prelude::*, px, transparent_black,
};
use smallvec::SmallVec;

use crate::components::label::{Label, LabelCommon};
use crate::components::stack::h_flex;
use crate::theme::TextSize;

// -------------------- Avatar --------------------

/// Avatar size, used by both [`Avatar`] and [`Facepile`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum AvatarSize {
    Small,
    #[default]
    Medium,
    Large,
}

impl AvatarSize {
    fn diameter(self) -> Pixels {
        match self {
            Self::Small => px(20.0),
            Self::Medium => px(28.0),
            Self::Large => px(36.0),
        }
    }

    fn text_size(self) -> TextSize {
        match self {
            Self::Small => TextSize::XSmall,
            Self::Medium => TextSize::Small,
            Self::Large => TextSize::Default,
        }
    }
}

/// A circular profile bubble.
///
/// Defaults to a monogram (the first character of `name` over a
/// hash-derived hue), so it stays visually consistent across renders even
/// without an image. Call [`Avatar::image`] to swap in a real picture; the
/// `name` is still stored for accessibility hints and as a fallback if the
/// image fails to load.
#[derive(IntoElement)]
#[must_use = "Avatar does nothing unless rendered"]
pub struct Avatar {
    name: SharedString,
    size: AvatarSize,
    color_override: Option<Hsla>,
    image: Option<ImageSource>,
}

impl Avatar {
    pub fn new(name: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            size: AvatarSize::Medium,
            color_override: None,
            image: None,
        }
    }

    pub fn size(mut self, size: AvatarSize) -> Self {
        self.size = size;
        self
    }

    /// Override the auto-generated background color.
    pub fn color(mut self, color: Hsla) -> Self {
        self.color_override = Some(color);
        self
    }

    /// Render a real image instead of a monogram.
    ///
    /// Accepts any [`ImageSource`] - URLs, file paths, pre-loaded
    /// [`Arc<Image>`](gpui::Image), etc. See [`gpui::img`] for the full
    /// list of `From` impls.
    pub fn image(mut self, source: impl Into<ImageSource>) -> Self {
        self.image = Some(source.into());
        self
    }
}

/// Hash a name into a stable hue. Cheap, deterministic, no external crate.
fn hue_for(name: &str) -> f32 {
    let mut hash: u32 = 0;
    for byte in name.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u32);
    }
    (hash % 360) as f32 / 360.0
}

fn initial_of(name: &str) -> String {
    name.chars()
        .next()
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_else(|| "?".to_string())
}

impl RenderOnce for Avatar {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let Avatar {
            name,
            size,
            color_override,
            image,
        } = self;
        let diameter = size.diameter();
        let bg = color_override.unwrap_or_else(|| hsla(hue_for(&name), 0.55, 0.45, 1.0));
        // The hue-derived disk shows through any transparent regions of
        // the image and is what's visible during load, so we always paint
        // it. When `image` is set, the image is overlaid on top; otherwise
        // the monogram is drawn.
        let has_image = image.is_some();
        let initial: SharedString = initial_of(&name).into();
        div()
            .size(diameter)
            .rounded_full()
            .bg(bg)
            .flex()
            .items_center()
            .justify_center()
            .when_some(image, |this, source| {
                this.child(img(source).size(diameter).rounded_full().bg(bg))
            })
            .when(!has_image, |this| {
                this.child(
                    Label::new(initial)
                        .size(size.text_size())
                        .color(Color::Custom(hsla(0.0, 0.0, 1.0, 1.0))),
                )
            })
    }
}

// -------------------- Facepile --------------------

/// A horizontal stack of overlapping [`Avatar`]s.
///
/// Avatars are placed with negative left margins so they overlap by ~30% of
/// their diameter. Pass them in display order; the last avatar is drawn on
/// top.
#[derive(IntoElement)]
#[must_use = "Facepile does nothing unless rendered"]
pub struct Facepile {
    avatars: SmallVec<[Avatar; 4]>,
}

impl Facepile {
    pub fn new() -> Self {
        Self {
            avatars: SmallVec::new(),
        }
    }

    pub fn push(mut self, avatar: Avatar) -> Self {
        self.avatars.push(avatar);
        self
    }
}

impl Default for Facepile {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for Facepile {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let avatars = self.avatars;
        h_flex().children(
            avatars
                .into_iter()
                .enumerate()
                .map(|(i, avatar)| div().when(i > 0, |this| this.ml(px(-8.0))).child(avatar)),
        )
    }
}

// -------------------- Chip --------------------

/// Visual style of a [`Chip`]. Maps to a label color and background tint.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ChipStyle {
    #[default]
    Default,
    Accent,
    Success,
    Warning,
    Error,
    Info,
}

/// Display size of a [`Chip`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ChipSize {
    /// Compact - `TextSize::XSmall`, tight padding.
    Small,
    /// Default - `TextSize::Small`, standard padding.
    #[default]
    Medium,
}

/// A small rounded label, useful for tags / pills / status markers.
///
/// Chips default to a fully rounded (pill) shape. Use `.outline(true)` for
/// a transparent-background variant with a colored border.
#[derive(IntoElement)]
#[must_use = "Chip does nothing unless rendered"]
pub struct Chip {
    label: SharedString,
    style: ChipStyle,
    size: ChipSize,
    outline: bool,
}

impl Chip {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            style: ChipStyle::Default,
            size: ChipSize::default(),
            outline: false,
        }
    }

    pub fn style(mut self, style: ChipStyle) -> Self {
        self.style = style;
        self
    }

    pub fn size(mut self, size: ChipSize) -> Self {
        self.size = size;
        self
    }

    /// Render as an outlined chip (transparent background, colored border).
    pub fn outline(mut self, outline: bool) -> Self {
        self.outline = outline;
        self
    }
}

impl RenderOnce for Chip {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors();
        let status = &colors.status;

        let label_color = match self.style {
            ChipStyle::Default => Color::Default,
            ChipStyle::Accent => Color::Accent,
            ChipStyle::Success => Color::Success,
            ChipStyle::Warning => Color::Warning,
            ChipStyle::Error => Color::Error,
            ChipStyle::Info => Color::Info,
        };

        // In outline mode the background is transparent and the border
        // takes the label's resolved color. In filled mode the status
        // flavors pull bg + border from StatusColors; Default and Accent
        // keep the neutral element background.
        let (bg, border) = if self.outline {
            (transparent_black(), label_color.hsla(colors))
        } else {
            match self.style {
                ChipStyle::Success => (status.success_background, status.success_border),
                ChipStyle::Warning => (status.warning_background, status.warning_border),
                ChipStyle::Error => (status.error_background, status.error_border),
                ChipStyle::Info => (status.info_background, status.info_border),
                _ => (colors.element_background, colors.border),
            }
        };

        let (label_size, px_x, px_y) = match self.size {
            ChipSize::Small => (TextSize::XSmall, Spacing::Small, Spacing::None),
            ChipSize::Medium => (TextSize::Small, Spacing::Medium, Spacing::None),
        };

        div()
            .flex_none()
            .self_start()
            .px(px_x.pixels())
            .py(px_y.pixels())
            .rounded(Radius::Small.pixels())
            .border_1()
            .border_color(border)
            .bg(bg)
            .child(Label::new(self.label).size(label_size).color(label_color))
    }
}

// -------------------- CountBadge --------------------

/// A small numeric badge. Counts above 99 render as `99+`.
#[derive(IntoElement)]
#[must_use = "CountBadge does nothing unless rendered"]
pub struct CountBadge {
    count: usize,
}

impl CountBadge {
    pub fn new(count: usize) -> Self {
        Self { count }
    }
}

impl RenderOnce for CountBadge {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors();
        let label: SharedString = if self.count > 99 {
            "99+".into()
        } else {
            self.count.to_string().into()
        };
        // Make the badge wider than tall but never narrower than it is tall,
        // so single-digit counts stay circular.
        div()
            .min_w(px(18.0))
            .h(px(18.0))
            .px(px(5.0))
            .rounded(Radius::Full.pixels())
            .bg(colors.accent)
            .flex()
            .items_center()
            .justify_center()
            .child(
                Label::new(label)
                    .size(TextSize::XSmall)
                    .color(Color::Custom(colors.background)),
            )
    }
}

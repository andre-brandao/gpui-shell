//! Motion tokens: the springs and entrances every surface shares, so a chip
//! in the bar and a card in a popup move with one hand.
//!
//! Springs hold position and velocity under the element's id, so a retarget
//! mid-flight bends the motion instead of restarting it. Entrances play once,
//! the first frame their id renders. Both honour `App::reduce_motion`.

use std::time::Duration;

use gpui::{
    Animation, AnimationElement, AnimationExt, AnimationPhase, ElementId, Hsla, Interpolate,
    IntoElement, Pixels, Rgba, SpringAnimation, SpringConfig, SpringTarget, Styled, ease_out_quint,
    px,
};

/// How a value gets to where it is going, as damping ratios.
pub struct Motion;

impl Motion {
    /// Decisive, no visible overshoot (ζ 0.8, settles in ~250ms): selection,
    /// levels and anything that changes layout.
    pub const SNAPPY: SpringConfig = SpringConfig::new(400.0, 32.0, 1.0);
    /// Soft landing (ζ 0.9, ~350ms): colour and emphasis that should not pull
    /// the eye.
    pub const GENTLE: SpringConfig = SpringConfig::new(220.0, 27.0, 1.0);
    /// A touch of overshoot (ζ 0.55): direct manipulation, hover and press.
    pub const LIVELY: SpringConfig = SpringConfig::new(380.0, 21.0, 1.0);
    /// How long a one-shot entrance takes.
    pub const ENTER: Duration = Duration::from_millis(180);
    /// How far an entering element travels into place.
    pub const ENTER_DISTANCE: Pixels = px(6.0);
}

/// Settling tolerance for springs on a 0..1 phase: half a percent is below
/// what a pixel shows, and ends the frame loop a third of a second sooner
/// than gpui's default.
const PHASE_EPSILON: f32 = 0.005;

/// Builds a spring from a preset: `Motion::SNAPPY.to(is_active)`.
pub trait SpringConfigExt {
    /// A spring toward `target`, driven through
    /// [`AnimationExt::with_spring`](gpui::AnimationExt::with_spring).
    fn to<T: SpringTarget>(self, target: T) -> SpringAnimation<T>;
}

impl SpringConfigExt for SpringConfig {
    fn to<T: SpringTarget>(self, target: T) -> SpringAnimation<T> {
        SpringAnimation::new(self)
            .to(target)
            .with_epsilon(PHASE_EPSILON)
    }
}

/// `from` at phase 0, `to` at phase 1, blended in RGB so a grey never picks
/// up a hue on its way to a colour. Clamped: colours do not overshoot.
pub fn mix(phase: AnimationPhase, from: Hsla, to: Hsla) -> Hsla {
    let t = phase.0.clamp(0.0, 1.0);
    Rgba::interpolate(from.into(), to.into(), t).into()
}

/// Where an entering element comes from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Enter {
    /// Fades in where it stands.
    #[default]
    InPlace,
    FromAbove,
    FromBelow,
    FromLeft,
    FromRight,
}

/// One-shot entrances for any styled element.
pub trait MotionExt: Styled + IntoElement + Sized + 'static {
    /// Fade and settle into place over [`Motion::ENTER`], the first frame
    /// `id` renders. Re-renders under the same id do not replay it, so key
    /// the id by what the element shows when a swap should read as one.
    fn enter(self, id: impl Into<ElementId>, from: Enter) -> AnimationElement<Self> {
        self.enter_over(id, from, Motion::ENTER, Motion::ENTER_DISTANCE)
    }

    /// [`enter`](Self::enter) with its own length and travel.
    fn enter_over(
        self,
        id: impl Into<ElementId>,
        from: Enter,
        duration: Duration,
        distance: Pixels,
    ) -> AnimationElement<Self> {
        self.with_animation(
            id,
            Animation::new(duration).with_easing(ease_out_quint()),
            move |this, t| {
                let left = distance * (1.0 - t);
                let this = this.opacity(t);
                match from {
                    Enter::InPlace => this,
                    Enter::FromAbove => this.relative().top(-left),
                    Enter::FromBelow => this.relative().top(left),
                    Enter::FromLeft => this.relative().left(-left),
                    Enter::FromRight => this.relative().left(left),
                }
            },
        )
    }
}

impl<E: Styled + IntoElement + 'static> MotionExt for E {}

#[cfg(test)]
mod tests {
    use super::*;

    fn damping_ratio(config: SpringConfig) -> f32 {
        config.canonical().1
    }

    #[test]
    fn presets_are_underdamped_but_settle() {
        assert!((0.75..0.85).contains(&damping_ratio(Motion::SNAPPY)));
        assert!((0.85..0.95).contains(&damping_ratio(Motion::GENTLE)));
        assert!((0.5..0.6).contains(&damping_ratio(Motion::LIVELY)));
    }

    #[test]
    fn mix_stays_between_its_ends() {
        let grey = gpui::hsla(0.0, 0.0, 0.5, 1.0);
        let blue = gpui::hsla(0.6, 0.8, 0.5, 1.0);
        let mid: Rgba = mix(AnimationPhase(0.5), grey, blue).into();
        let (g, b): (Rgba, Rgba) = (grey.into(), blue.into());
        for (m, lo, hi) in [(mid.r, g.r, b.r), (mid.g, g.g, b.g), (mid.b, g.b, b.b)] {
            assert!(m >= lo.min(hi) - 1e-5 && m <= lo.max(hi) + 1e-5);
        }
        assert_eq!(
            mix(AnimationPhase(1.3), grey, blue),
            mix(AnimationPhase(1.0), grey, blue)
        );
    }
}

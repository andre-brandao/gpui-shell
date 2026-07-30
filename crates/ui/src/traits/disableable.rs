/// Elements whose interactivity and visual style can be suppressed.
pub trait Disableable {
    fn disabled(self, disabled: bool) -> Self;
}

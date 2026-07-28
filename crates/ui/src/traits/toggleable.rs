/// Elements with two (or three, with [`ToggleState::Indeterminate`]) visual
/// states - checkboxes, toggle buttons, selectable list items.
pub trait Toggleable {
    fn toggle_state(self, state: impl Into<ToggleState>) -> Self;
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToggleState {
    #[default]
    Unselected,
    Indeterminate,
    Selected,
}

impl ToggleState {
    pub fn selected(self) -> bool {
        matches!(self, Self::Selected)
    }

    pub fn indeterminate(self) -> bool {
        matches!(self, Self::Indeterminate)
    }

    pub fn inverse(self) -> Self {
        match self {
            Self::Selected => Self::Unselected,
            Self::Unselected | Self::Indeterminate => Self::Selected,
        }
    }

    /// Build a state from "any-checked" / "all-checked" flags. The typical
    /// caller is a header checkbox summarizing a group of children: every
    /// child checked -> [`Selected`], none checked -> [`Unselected`], any
    /// in-between -> [`Indeterminate`].
    pub fn from_any_and_all(any_checked: bool, all_checked: bool) -> Self {
        match (any_checked, all_checked) {
            (true, true) => Self::Selected,
            (false, false) => Self::Unselected,
            _ => Self::Indeterminate,
        }
    }
}

impl From<bool> for ToggleState {
    fn from(value: bool) -> Self {
        if value {
            Self::Selected
        } else {
            Self::Unselected
        }
    }
}

impl From<Option<bool>> for ToggleState {
    fn from(value: Option<bool>) -> Self {
        match value {
            Some(true) => Self::Selected,
            Some(false) => Self::Unselected,
            None => Self::Indeterminate,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_only_true_for_selected() {
        assert!(ToggleState::Selected.selected());
        assert!(!ToggleState::Unselected.selected());
        assert!(!ToggleState::Indeterminate.selected());
    }

    #[test]
    fn indeterminate_only_true_for_indeterminate() {
        assert!(ToggleState::Indeterminate.indeterminate());
        assert!(!ToggleState::Selected.indeterminate());
        assert!(!ToggleState::Unselected.indeterminate());
    }

    #[test]
    fn inverse_round_trips_binary_states() {
        assert_eq!(ToggleState::Selected.inverse(), ToggleState::Unselected);
        assert_eq!(ToggleState::Unselected.inverse(), ToggleState::Selected);
    }

    #[test]
    fn inverse_collapses_indeterminate_to_selected() {
        // A tri-state checkbox typically resolves "?" to "checked" on click.
        assert_eq!(ToggleState::Indeterminate.inverse(), ToggleState::Selected);
    }

    #[test]
    fn from_bool() {
        assert_eq!(ToggleState::from(true), ToggleState::Selected);
        assert_eq!(ToggleState::from(false), ToggleState::Unselected);
    }

    #[test]
    fn default_is_unselected() {
        assert_eq!(ToggleState::default(), ToggleState::Unselected);
    }

    #[test]
    fn from_any_and_all_collapses_to_indeterminate_when_partial() {
        assert_eq!(
            ToggleState::from_any_and_all(true, true),
            ToggleState::Selected
        );
        assert_eq!(
            ToggleState::from_any_and_all(false, false),
            ToggleState::Unselected
        );
        assert_eq!(
            ToggleState::from_any_and_all(true, false),
            ToggleState::Indeterminate
        );
    }

    #[test]
    fn from_option_bool_maps_none_to_indeterminate() {
        assert_eq!(ToggleState::from(Some(true)), ToggleState::Selected);
        assert_eq!(ToggleState::from(Some(false)), ToggleState::Unselected);
        assert_eq!(ToggleState::from(None::<bool>), ToggleState::Indeterminate);
    }
}

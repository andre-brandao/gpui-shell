//! Shared type aliases for component event handlers.

use std::rc::Rc;

use gpui::{AnyView, App, ClickEvent, MouseDownEvent, Window};

use crate::traits::ToggleState;

/// Mouse click: buttons, list items, tabs, menu entries, backdrop dismisses.
pub type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// String payload: text field `on_change` / `on_submit`.
pub type StringHandler = Rc<dyn Fn(&str, &mut Window, &mut App) + 'static>;

/// Toggle flip. Receives the state *after* the flip.
pub type ToggleHandler = Rc<dyn Fn(&ToggleState, &mut Window, &mut App) + 'static>;

/// Overlay close request. No payload - fired from both backdrop click and Escape.
pub type DismissHandler = Rc<dyn Fn(&mut Window, &mut App) + 'static>;

/// Hover enter (`true`) / leave (`false`), matching gpui's `Div::on_hover`.
pub type HoverHandler = Rc<dyn Fn(&bool, &mut Window, &mut App) + 'static>;

/// Raw mouse-down, for callers needing more than [`ClickHandler`] carries.
pub type MouseDownHandler = Rc<dyn Fn(&MouseDownEvent, &mut Window, &mut App) + 'static>;

/// `f64` payload: stepper value changes.
pub type F64Handler = Rc<dyn Fn(f64, &mut Window, &mut App) + 'static>;

/// `f32` payload: slider value changes.
pub type F32Handler = Rc<dyn Fn(f32, &mut Window, &mut App) + 'static>;

/// `usize` payload: page or item index.
pub type UsizeHandler = Rc<dyn Fn(usize, &mut Window, &mut App) + 'static>;

/// Lazily builds a tooltip view, invoked at hover time.
pub type TooltipBuilder = Rc<dyn Fn(&mut Window, &mut App) -> AnyView + 'static>;

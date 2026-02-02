//! Widget registry for dynamic widget creation.
//!
//! This module provides a registry pattern that allows widgets to be created
//! by name, enabling configuration-driven bar layouts.

use gpui::{AnyElement, Context, Entity, prelude::*};

use services::Services;

use super::{Battery, Clock, KeyboardLayout, LauncherBtn, Settings, SysInfo, Tray, Workspaces};

/// Wrapper enum for all possible widget types.
///
/// Each variant holds an Entity handle to a specific widget type.
/// This allows heterogeneous widgets to be stored in collections
/// and rendered uniformly.
pub enum Widget {
    Clock(Entity<Clock>),
    Battery(Entity<Battery>),
    Workspaces(Entity<Workspaces>),
    KeyboardLayout(Entity<KeyboardLayout>),
    Tray(Entity<Tray>),
    SysInfo(Entity<SysInfo>),
    LauncherBtn(Entity<LauncherBtn>),
    Settings(Entity<Settings>),
}

impl Widget {
    /// Render the widget to an AnyElement.
    ///
    /// This allows uniform rendering regardless of the underlying widget type.
    pub fn render(&self) -> AnyElement {
        match self {
            Widget::Clock(e) => e.clone().into_any_element(),
            Widget::Battery(e) => e.clone().into_any_element(),
            Widget::Workspaces(e) => e.clone().into_any_element(),
            Widget::KeyboardLayout(e) => e.clone().into_any_element(),
            Widget::Tray(e) => e.clone().into_any_element(),
            Widget::SysInfo(e) => e.clone().into_any_element(),
            Widget::LauncherBtn(e) => e.clone().into_any_element(),
            Widget::Settings(e) => e.clone().into_any_element(),
        }
    }

    /// Create a widget by name.
    ///
    /// Returns `None` if the widget name is unknown.
    ///
    /// # Arguments
    /// * `name` - The widget name (e.g., "Clock", "Battery", "Settings")
    /// * `services` - Shared services for widgets that need them
    /// * `cx` - The GPUI context
    pub fn create<V: 'static>(
        name: &str,
        services: &Services,
        cx: &mut Context<V>,
    ) -> Option<Widget> {
        match name {
            "Clock" => Some(Widget::Clock(cx.new(Clock::new))),
            "Battery" => Some(Widget::Battery(
                cx.new(|cx| Battery::new(services.upower.clone(), cx)),
            )),
            "Workspaces" => Some(Widget::Workspaces(
                cx.new(|cx| Workspaces::new(services.compositor.clone(), cx)),
            )),
            "KeyboardLayout" => {
                Some(Widget::KeyboardLayout(cx.new(|cx| {
                    KeyboardLayout::new(services.compositor.clone(), cx)
                })))
            }
            "Systray" | "Tray" => Some(Widget::Tray(
                cx.new(|cx| Tray::new(services.tray.clone(), cx)),
            )),
            "SysInfo" => Some(Widget::SysInfo(
                cx.new(|cx| SysInfo::new(services.sysinfo.clone(), cx)),
            )),
            "LauncherBtn" | "Launcher" => Some(Widget::LauncherBtn(
                cx.new(|cx| LauncherBtn::new(services.clone(), cx)),
            )),
            "Settings" | "Info" | "ControlCenter" => Some(Widget::Settings(
                cx.new(|cx| Settings::new(services.clone(), cx)),
            )),
            _ => {
                tracing::warn!("Unknown widget: {}", name);
                None
            }
        }
    }

    /// Create multiple widgets from a list of names.
    ///
    /// Unknown widget names are silently filtered out (with a warning log).
    ///
    /// # Arguments
    /// * `names` - Slice of widget names to create
    /// * `services` - Shared services for widgets that need them
    /// * `cx` - The GPUI context
    pub fn create_many<V: 'static>(
        names: &[String],
        services: &Services,
        cx: &mut Context<V>,
    ) -> Vec<Widget> {
        names
            .iter()
            .filter_map(|name| Widget::create(name, services, cx))
            .collect()
    }
}

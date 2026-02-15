//! Widget registry for dynamic widget creation.
//!
//! This module provides a registry pattern that allows widgets to be created
//! by name, enabling configuration-driven bar layouts.

use gpui::{AnyElement, Context, Entity, prelude::*};

use crate::config::WidgetConfig;

use super::{
    ActiveWindow, Battery, Clock, KeyboardLayout, LauncherBtn, Settings, SysInfo, Tray, Workspaces,
};

/// Wrapper enum for all possible widget types.
///
/// Each variant holds an Entity handle to a specific widget type.
/// This allows heterogeneous widgets to be stored in collections
/// and rendered uniformly.
pub enum Widget {
    ActiveWindow(Entity<ActiveWindow>),
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
            Widget::ActiveWindow(e) => e.clone().into_any_element(),
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
    pub fn create<V: 'static>(name: &str, cx: &mut Context<V>) -> Option<Widget> {
        match name {
            "ActiveWindow" | "WindowTitle" => Some(Widget::ActiveWindow(cx.new(ActiveWindow::new))),
            "Clock" => Some(Widget::Clock(cx.new(Clock::new))),
            "Battery" => Some(Widget::Battery(cx.new(Battery::new))),
            "Workspaces" => Some(Widget::Workspaces(cx.new(Workspaces::new))),
            "KeyboardLayout" => Some(Widget::KeyboardLayout(cx.new(KeyboardLayout::new))),
            "Systray" | "Tray" => Some(Widget::Tray(cx.new(Tray::new))),
            "SysInfo" => Some(Widget::SysInfo(cx.new(SysInfo::new))),
            "LauncherBtn" | "Launcher" => Some(Widget::LauncherBtn(cx.new(LauncherBtn::new))),
            "Settings" | "Info" | "ControlCenter" => Some(Widget::Settings(cx.new(Settings::new))),
            _ => {
                tracing::warn!("Unknown widget: {}", name);
                None
            }
        }
    }

    /// Create multiple widgets from a config list.
    pub fn create_many<V: 'static>(widgets: &[WidgetConfig], cx: &mut Context<V>) -> Vec<Widget> {
        widgets
            .iter()
            .filter_map(|widget| Widget::create(&widget.name, cx))
            .collect()
    }
}

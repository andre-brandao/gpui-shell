//! Widget registry for dynamic widget creation using Zed UI components.
//!
//! Mirrors the previous factory pattern while keeping services as a
//! simple cloneable value (no `Entity` wrapping).

use gpui::{AnyElement, Context, Entity, prelude::*};
use services::Services;

use super::{
    ActiveWindow, Battery, Clock, KeyboardLayout, LauncherBtn, Settings, SysInfo, Tray, Workspaces,
};

/// Wrapper enum for all widget types.
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
    /// Render to an `AnyElement`, allowing uniform storage.
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
    pub fn create<V: 'static>(
        name: &str,
        services: &Services,
        cx: &mut Context<V>,
    ) -> Option<Widget> {
        match name {
            "ActiveWindow" | "WindowTitle" => Some(Widget::ActiveWindow(
                cx.new(|cx| ActiveWindow::new(services.clone(), cx)),
            )),
            "Clock" => Some(Widget::Clock(cx.new(Clock::new))),
            "Battery" => Some(Widget::Battery(
                cx.new(|cx| Battery::new(services.clone(), cx)),
            )),
            "Workspaces" => Some(Widget::Workspaces(
                cx.new(|cx| Workspaces::new(services.clone(), cx)),
            )),
            "KeyboardLayout" => Some(Widget::KeyboardLayout(
                cx.new(|cx| KeyboardLayout::new(services.clone(), cx)),
            )),
            "Systray" | "Tray" => Some(Widget::Tray(cx.new(|cx| Tray::new(services.clone(), cx)))),
            "SysInfo" => Some(Widget::SysInfo(
                cx.new(|cx| SysInfo::new(services.clone(), cx)),
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

use crate::services::Services;
use gpui::{AnyElement, Context, Entity, prelude::*};

use super::{Clock, Info, LauncherBtn, Systray, Workspaces};

/// Wrapper enum for all possible widget types.
pub enum Widget {
    LauncherBtn(Entity<LauncherBtn>),
    Workspaces(Entity<Workspaces>),
    Clock(Entity<Clock>),
    Systray(Entity<Systray>),
    Info(Entity<Info>),
}

impl Widget {
    /// Render the widget to an AnyElement.
    pub fn render(&self) -> AnyElement {
        match self {
            Widget::LauncherBtn(e) => e.clone().into_any_element(),
            Widget::Workspaces(e) => e.clone().into_any_element(),
            Widget::Clock(e) => e.clone().into_any_element(),
            Widget::Systray(e) => e.clone().into_any_element(),
            Widget::Info(e) => e.clone().into_any_element(),
        }
    }

    /// Create a widget by name.
    /// Returns None if the widget name is unknown.
    pub fn create<V: 'static>(
        name: &str,
        services: &Services,
        cx: &mut Context<V>,
    ) -> Option<Widget> {
        match name {
            "LauncherBtn" => Some(Widget::LauncherBtn(
                cx.new(|cx| LauncherBtn::with_services(services.clone(), cx)),
            )),
            "Workspaces" => Some(Widget::Workspaces(
                cx.new(|cx| Workspaces::with_services(services.clone(), cx)),
            )),
            "Clock" => Some(Widget::Clock(cx.new(Clock::new))),
            "Systray" => Some(Widget::Systray(cx.new(Systray::new))),
            "Info" => Some(Widget::Info(
                cx.new(|cx| Info::with_services(services.clone(), cx)),
            )),
            _ => {
                eprintln!("Unknown widget: {}", name);
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

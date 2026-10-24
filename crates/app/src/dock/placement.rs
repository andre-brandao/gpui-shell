use gpui::{App, Bounds, MouseDownEvent, Size, Window, layer_shell::Anchor, point, px};
use ui::Spacing;

use crate::windows::ShellWindows;

use super::{
    DOCK_CONTEXT_MENU_BORDER_WIDTH, DOCK_CONTEXT_MENU_GAP, DOCK_CONTEXT_MENU_ROW_HEIGHT,
    DOCK_CONTEXT_MENU_TEXT_LAYOUT_HEADROOM, DOCK_CONTEXT_MENU_VERTICAL_PADDING,
    DOCK_CONTEXT_MENU_WIDTH,
};

pub(super) fn dock_context_menu_height(action_count: usize) -> f32 {
    let gaps = action_count.saturating_sub(1) as f32 * DOCK_CONTEXT_MENU_GAP;
    let content_height = action_count as f32 * DOCK_CONTEXT_MENU_ROW_HEIGHT;
    let vertical_padding = 2.0 * DOCK_CONTEXT_MENU_VERTICAL_PADDING;
    let borders = 2.0 * DOCK_CONTEXT_MENU_BORDER_WIDTH;

    content_height + gaps + vertical_padding + borders + DOCK_CONTEXT_MENU_TEXT_LAYOUT_HEADROOM
}

pub(super) fn dock_context_menu_size(action_count: usize) -> Size<gpui::Pixels> {
    Size::new(
        px(DOCK_CONTEXT_MENU_WIDTH),
        px(dock_context_menu_height(action_count)),
    )
}

pub(super) struct DockPanelPlacement {
    pub(super) anchor: Anchor,
    pub(super) margin: (f32, f32, f32, f32),
}

pub(super) fn dock_panel_placement_from_dock_click(
    dock_position: crate::bar::config::BarPosition,
    dock_click: gpui::Point<gpui::Pixels>,
    dock_size: Size<gpui::Pixels>,
    menu_size: Size<gpui::Pixels>,
    display_bounds: Bounds<gpui::Pixels>,
    usable_bounds: Bounds<gpui::Pixels>,
) -> DockPanelPlacement {
    let display_width: f32 = display_bounds.size.width.into();
    let display_height: f32 = display_bounds.size.height.into();
    let dock_width: f32 = dock_size.width.into();
    let dock_height: f32 = dock_size.height.into();
    let menu_width: f32 = menu_size.width.into();
    let menu_height: f32 = menu_size.height.into();

    let usable_left: f32 = (usable_bounds.origin.x - display_bounds.origin.x).into();
    let usable_top: f32 = (usable_bounds.origin.y - display_bounds.origin.y).into();
    let usable_right: f32 =
        (usable_bounds.origin.x + usable_bounds.size.width - display_bounds.origin.x).into();
    let usable_bottom: f32 =
        (usable_bounds.origin.y + usable_bounds.size.height - display_bounds.origin.y).into();

    let min_x = usable_left;
    let max_x = (usable_right - menu_width).max(min_x);
    let min_y = usable_top;
    let max_y = (usable_bottom - menu_height).max(min_y);

    let (dock_x, dock_y) = match dock_position {
        crate::bar::config::BarPosition::Top => (
            (display_width - dock_width).max(0.0) / 2.0,
            Spacing::Medium.value(),
        ),
        crate::bar::config::BarPosition::Bottom => (
            (display_width - dock_width).max(0.0) / 2.0,
            display_height - dock_height - Spacing::Medium.value(),
        ),
        crate::bar::config::BarPosition::Left => (
            Spacing::Medium.value(),
            (display_height - dock_height).max(0.0) / 2.0,
        ),
        crate::bar::config::BarPosition::Right => (
            display_width - dock_width - Spacing::Medium.value(),
            (display_height - dock_height).max(0.0) / 2.0,
        ),
    };

    let click_x: f32 = dock_x + Into::<f32>::into(dock_click.x);
    let click_y: f32 = dock_y + Into::<f32>::into(dock_click.y);

    if dock_position == crate::bar::config::BarPosition::Bottom {
        let origin_x = click_x - menu_width / 2.0;
        let bottom = display_height - dock_y;
        let max_bottom = (display_height - min_y - menu_height).max(0.0);
        return DockPanelPlacement {
            anchor: Anchor::BOTTOM | Anchor::LEFT,
            margin: (
                0.0,
                0.0,
                clamp_dock_menu_origin(bottom, 0.0, max_bottom),
                clamp_dock_menu_origin(origin_x, min_x, max_x),
            ),
        };
    }

    let (origin_x, origin_y) = match dock_position {
        crate::bar::config::BarPosition::Top => (click_x - menu_width / 2.0, dock_y + dock_height),
        crate::bar::config::BarPosition::Bottom => unreachable!("handled above"),
        crate::bar::config::BarPosition::Left => (dock_x + dock_width, click_y - menu_height / 2.0),
        crate::bar::config::BarPosition::Right => {
            (dock_x - menu_width, click_y - menu_height / 2.0)
        }
    };

    DockPanelPlacement {
        anchor: Anchor::TOP | Anchor::LEFT,
        margin: (
            clamp_dock_menu_origin(origin_y, min_y, max_y),
            0.0,
            0.0,
            clamp_dock_menu_origin(origin_x, min_x, max_x),
        ),
    }
}

pub(super) fn dock_panel_placement_from_event(
    dock_position: crate::bar::config::BarPosition,
    event: &MouseDownEvent,
    window: &Window,
    cx: &App,
    dock_size: Size<gpui::Pixels>,
    menu_size: Size<gpui::Pixels>,
) -> DockPanelPlacement {
    let (display_bounds, usable_bounds) = ShellWindows::display(cx, window)
        .map(|display| (display.bounds(), display.visible_bounds()))
        .unwrap_or_else(|| {
            let bounds = Bounds::new(point(px(0.0), px(0.0)), window.viewport_size());
            (bounds, bounds)
        });

    dock_panel_placement_from_dock_click(
        dock_position,
        event.position,
        dock_size,
        menu_size,
        display_bounds,
        usable_bounds,
    )
}

fn clamp_dock_menu_origin(value: f32, min: f32, max: f32) -> f32 {
    value.clamp(min, max)
}

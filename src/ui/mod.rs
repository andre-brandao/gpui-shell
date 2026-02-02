mod control_center;
mod panel;

pub use control_center::ControlCenter;
pub use panel::{
    PanelConfig, active_panel_id, close_panel, is_panel_open, panel_container, toggle_panel,
};

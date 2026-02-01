mod battery;
mod clock;
mod workspaces;

pub use battery::{battery, get_battery_percentage};
pub use clock::clock;
pub use workspaces::{WorkspaceInfo, fetch_workspaces, workspaces};

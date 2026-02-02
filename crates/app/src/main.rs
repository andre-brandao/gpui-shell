//! GPUi Shell - A Wayland status bar built with GPUI.

use assets::Assets;
use gpui::Application;
use services::Services;
use tracing_subscriber::EnvFilter;

mod bar;
pub mod control_center;
pub mod launcher;
mod panel;
pub mod widgets;

#[tokio::main]
async fn main() {
    #[cfg(not(target_os = "linux"))]
    compile_error!("This application requires a Linux system with Wayland.");

    // Initialize tracing with RUST_LOG env var support
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // Initialize services before starting GPUI
    let services = Services::new()
        .await
        .expect("Failed to initialize services");

    // Create and run the GPUI application
    Application::new().with_assets(Assets {}).run(|cx| {
        // Register launcher keybindings
        launcher::register_keybindings(cx);

        // Open the status bar
        bar::open(services, cx);
    });
}

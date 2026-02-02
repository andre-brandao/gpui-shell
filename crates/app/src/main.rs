//! GPUi Shell - A Wayland status bar built with GPUI.

use assets::Assets;
use gpui::Application;
use services::Services;

mod bar;
mod widgets;

#[tokio::main]
async fn main() {
    #[cfg(not(target_os = "linux"))]
    compile_error!("This application requires a Linux system with Wayland.");

    // Initialize services before starting GPUI
    let services = Services::new()
        .await
        .expect("Failed to initialize services");

    // Create and run the GPUI application
    Application::new().with_assets(Assets {}).run(|cx| {
        bar::open(services, cx);
    });
}

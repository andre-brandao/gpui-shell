//! GPUi Shell - A Wayland status bar built with GPUI.
//!
//! Supports single-instance mode: if the app is already running,
//! subsequent invocations will signal the existing instance to open the launcher.
//!
//! Usage:
//!   gpuishell              - Start the shell or open launcher if already running
//!   gpuishell --input "x"  - Open launcher with prefilled input

use assets::Assets;
use gpui::Application;
use services::{InstanceResult, Services, ShellSubscriber};
use tracing_subscriber::EnvFilter;

mod args;
// mod bar;  // Temporarily disabled - uses old UI module
mod bar2;
// pub mod osd;  // Temporarily disabled - uses old UI module
mod panel;
// pub mod widgets;  // Temporarily disabled - uses old UI module

use args::Args;

#[tokio::main]
async fn main() {
    #[cfg(not(target_os = "linux"))]
    compile_error!("This application requires a Linux system with Wayland.");

    // Initialize tracing with RUST_LOG env var support
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // Initialize services (requires async)
    let services = Services::new()
        .await
        .expect("Failed to initialize services");

    // Create and run the GPUI application
    let app = Application::new().with_assets(Assets {});

    app.run(move |cx| {
        // Initialize Zed's theme system
        bar2::init_theme(cx);

        // Open the status bar (use bar2 for PoC testing)
        // bar::open(services.clone(), cx);
        bar2::open(cx);

        // Start the OSD listener for volume/brightness changes
        // osd::start(services.clone(), osd::OsdPosition::Right, cx);
        let _ = services; // suppress unused warning
    })
}

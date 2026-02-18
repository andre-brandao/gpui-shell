//! GPUi Shell - A Wayland status bar built with GPUI.
//!
//! Supports single-instance mode: if the app is already running,
//! subsequent invocations will signal the existing instance to open the launcher.
//!
//! Usage:
//!   gpuishell              - Start the shell or open launcher if already running
//!   gpuishell --input "x"  - Open launcher with prefilled input

use crate::ipc::IpcSubscriber;
use assets::Assets;
use gpui::Application;
use tracing_subscriber::EnvFilter;

mod args;
mod bar;
pub mod config;
pub mod control_center;
mod ipc;
mod keybinds;
pub mod launcher;
pub mod notification;
pub mod osd;
mod panel;
pub mod state;

#[tokio::main]
async fn main() {
    #[cfg(not(target_os = "linux"))]
    compile_error!("This application requires a Linux system with Wayland.");

    // Initialize tracing with RUST_LOG env var support
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // Parse command-line arguments
    let args = args::Args::parse();

    // Try to acquire single-instance lock or signal existing instance.
    // Secondary path exits immediately after signaling the primary instance.
    let Some(ipc) = IpcSubscriber::init(&args) else {
        return;
    };

    // Initialize services (requires async)
    let services = state::init_services()
        .await
        .expect("Failed to initialize services");

    // Create and run the GPUI application
    let app = Application::new().with_assets(Assets {});
    app.run(move |cx| {
        config::Config::init(cx);
        state::AppState::init(services, cx);

        // Register keybindings
        keybinds::register(cx);

        bar::init(cx);
        notification::init(cx);
        osd::init(cx);
        launcher::init(cx);

        ipc.start(cx);
    });
}

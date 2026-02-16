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
use services::{Services, ShellSubscriber};
use tracing_subscriber::EnvFilter;

mod args;
mod bar;
pub mod config;
pub mod control_center;
pub mod launcher;
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
    let Some(mut shell) = ShellSubscriber::init(args.input) else {
        return;
    };

    // Initialize services (requires async)
    let services = Services::new()
        .await
        .expect("Failed to initialize services");

    // Start the shell listener now that we have a runtime context
    let shell_receiver = shell.start_listener();

    // Create and run the GPUI application
    let app = Application::new().with_assets(Assets {});

    app.run(move |cx| {
        config::Config::init(cx);
        state::AppState::init(services.clone(), cx);

        // Register keybindings
        launcher::register_keybindings(cx);
        control_center::ControlCenter::register_keybindings(cx);

        bar::init(cx);
        osd::init(cx);
        launcher::init(shell_receiver, cx);
    });
}

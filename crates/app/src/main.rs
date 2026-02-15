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
mod bar;
pub mod config;
pub mod control_center;
pub mod launcher;
pub mod osd;
mod panel;
pub mod widgets;

use args::Args;

#[tokio::main]
async fn main() {
    #[cfg(not(target_os = "linux"))]
    compile_error!("This application requires a Linux system with Wayland.");

    // Initialize tracing with RUST_LOG env var support
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // Parse command-line arguments
    let args = Args::parse();

    // Try to acquire single-instance lock or signal existing instance
    // The secondary path (signaling existing instance) is fast and exits immediately
    let mut shell = match ShellSubscriber::acquire(args.input) {
        InstanceResult::Primary(subscriber) => subscriber,
        InstanceResult::Secondary => {
            // Another instance is running and was signaled, exit immediately
            return;
        }
        InstanceResult::Error(e) => {
            tracing::error!("Shell service error: {}", e);
            // Continue without single-instance support
            tracing::warn!("Running without single-instance support");
            match ShellSubscriber::acquire(None) {
                InstanceResult::Primary(s) => s,
                _ => panic!("Failed to acquire shell service"),
            }
        }
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
        // Initialize the global theme
        ui::Theme::init(cx);
        // Initialize the global app config
        config::Config::init(cx);

        // Register keybindings
        launcher::register_keybindings(cx);
        control_center::ControlCenter::register_keybindings(cx);

        // Open the status bar
        bar::open(services.clone(), cx);

        // Start the OSD listener for volume/brightness changes
        osd::start(services.clone(), osd::OsdPosition::Right, cx);

        // Listen for launcher requests from other instances
        let services_for_shell = services.clone();
        let mut receiver = shell_receiver;
        cx.spawn(async move |cx| {
            tracing::info!("Shell request listener started");

            while let Some(request) = receiver.recv().await {
                tracing::info!(
                    "Processing launcher request: id={}, input={:?}",
                    request.id,
                    request.input
                );

                let services = services_for_shell.clone();
                let input = request.input;

                cx.update(move |cx| {
                    tracing::info!("Toggling launcher from IPC: {:?}", input);
                    launcher::toggle_from_ipc(services, input, cx);
                });
            }

            tracing::warn!("Shell request listener ended unexpectedly");
        })
        .detach();
    });
}

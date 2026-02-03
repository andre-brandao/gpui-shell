//! GPUi Shell - A Wayland status bar built with GPUI.
//!
//! Supports single-instance mode: if the app is already running,
//! subsequent invocations will signal the existing instance to open the launcher.
//!
//! Usage:
//!   gpuishell              - Start the shell or open launcher if already running
//!   gpuishell --input "x"  - Open launcher with prefilled input

use assets::Assets;
use futures_signals::signal::SignalExt;
use gpui::Application;
use services::{InstanceResult, Services, ShellSubscriber};
use tracing_subscriber::EnvFilter;

mod bar;
pub mod control_center;
pub mod launcher;
mod panel;
pub mod widgets;

/// Command-line arguments.
struct Args {
    /// Optional input to prefill in the launcher.
    input: Option<String>,
}

impl Args {
    fn parse() -> Self {
        let args: Vec<String> = std::env::args().collect();
        let mut input = None;

        let mut i = 1;
        while i < args.len() {
            if args[i] == "--input" || args[i] == "-i" {
                if i + 1 < args.len() {
                    input = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Error: --input requires a value");
                    std::process::exit(1);
                }
            } else {
                i += 1;
            }
        }

        Args { input }
    }
}

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
    let shell = match ShellSubscriber::acquire(args.input).await {
        InstanceResult::Primary(subscriber) => subscriber,
        InstanceResult::Secondary => {
            // Another instance is running and was signaled, exit
            return;
        }
        InstanceResult::Error(e) => {
            tracing::error!("Shell service error: {}", e);
            // Continue without single-instance support
            // This shouldn't normally happen, but we can still run
            tracing::warn!("Running without single-instance support");
            ShellSubscriber::acquire(None)
                .await
                .unwrap_primary_or_panic()
        }
    };

    // Initialize services before starting GPUI
    let services = Services::new()
        .await
        .expect("Failed to initialize services");

    // Create and run the GPUI application
    Application::new().with_assets(Assets {}).run(move |cx| {
        // Register keybindings
        launcher::register_keybindings(cx);
        control_center::ControlCenter::register_keybindings(cx);

        // Open the status bar
        bar::open(services.clone(), cx);

        // Subscribe to shell service for launcher requests from other instances
        let services_for_shell = services.clone();
        let shell_clone = shell.clone();
        cx.spawn(async move |cx| {
            use futures_util::StreamExt;

            let mut signal = shell_clone.subscribe().to_stream();
            tracing::info!("Shell signal listener started");

            while let Some(data) = signal.next().await {
                tracing::debug!("Shell signal received: {:?}", data.launcher_request);
                if let Some(request) = data.launcher_request {
                    tracing::info!(
                        "Processing launcher request: id={}, input={:?}",
                        request.id,
                        request.input
                    );
                    let services = services_for_shell.clone();
                    let input = request.input;
                    let shell = shell_clone.clone();

                    let _ = cx.update(move |cx| {
                        tracing::info!("Opening launcher with input: {:?}", input);
                        launcher::open_with_input(services, input, cx);
                        // Clear the request after handling
                        shell.clear_request();
                    });
                }
            }
            tracing::warn!("Shell signal listener ended unexpectedly");
        })
        .detach();
    });
}

/// Extension trait for InstanceResult.
trait InstanceResultExt {
    fn unwrap_primary_or_panic(self) -> ShellSubscriber;
}

impl InstanceResultExt for InstanceResult {
    fn unwrap_primary_or_panic(self) -> ShellSubscriber {
        match self {
            InstanceResult::Primary(s) => s,
            _ => panic!("Expected primary instance"),
        }
    }
}

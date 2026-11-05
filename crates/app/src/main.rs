//! GPUi Shell - A Wayland status bar built with GPUI.
//!
//! Supports single-instance mode: if the app is already running,
//! subsequent invocations will signal the existing instance to open the launcher.
//!
//! Usage:
//!   gpuishell              - Start the shell or open launcher if already running
//!   gpuishell --input "x"  - Open launcher with prefilled input
//!   gpuishell --validate   - Check the config files and exit

use crate::ipc::IpcSubscriber;
use assets::Assets;
use gpui_platform::application;
use tracing_subscriber::EnvFilter;

mod args;
mod bar;
pub mod config;
pub mod control_center;
pub mod dock;
pub(crate) mod frame;
pub mod icons;
mod ipc;
mod keybinds;
pub mod launcher;
pub mod notification;
pub mod osd;
pub mod state;
mod windows;

#[tokio::main]
async fn main() {
    #[cfg(not(target_os = "linux"))]
    compile_error!("This application requires a Linux system with Wayland.");

    // Initialize tracing with RUST_LOG env var support
    tracing_subscriber::fmt()
        .with_env_filter(
            // gpui logs "window not found" for each event that reaches a closed surface.
            EnvFilter::builder().parse_lossy(format!(
                "error,gpui::window=off,{}",
                std::env::var("RUST_LOG").unwrap_or_default()
            )),
        )
        .init();

    // Parse command-line arguments
    let args = args::Args::parse();

    // Before the single-instance check: a running shell must not swallow this.
    if args.validate {
        std::process::exit(if config::validate() { 0 } else { 1 });
    }

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
    // gpui ships a no-op client, so `img(url)` (album art) needs a real one.
    let http_client = reqwest_client::ReqwestClient::user_agent("gpuishell")
        .expect("Failed to build the HTTP client");
    let app = application()
        .with_assets(Assets {})
        .with_http_client(std::sync::Arc::new(http_client));
    app.run(move |cx| {
        config::Config::init(cx);
        // Registers the TextField / Menu keybindings the component set needs.
        ui::init(cx);
        windows::ShellWindows::init(cx);
        state::AppState::init(services, cx);

        // Register keybindings
        keybinds::register(cx);

        bar::init(cx);
        frame::init(cx);
        dock::init(cx);
        notification::init(cx);
        osd::init(cx);
        launcher::init(cx);

        // Now that the notification service is up, say what didn't parse.
        config::report_load_errors(cx);

        ipc.start(cx);
    });
}

use assets::Assets;
use gpui::Application;

fn main() {
    #[cfg(not(target_os = "linux"))]
    compile_error!("This application requires a Linux system with Wayland.");

    let app = Application::new().with_assets(Assets {});

    app.run(|_cx| {
        // Create all services once at startup
        // let services = services::Services::new(cx);

        // Register launcher keybindings
        // launcher::register_keybindings(cx);

        // // Open the bar window with shared services
        // bar::open(services.clone(), cx);

        // // Open the launcher on startup
        // launcher::open(services.clone(), cx);
    });
}

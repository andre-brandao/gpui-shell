use assets::Assets;
use gpui::Application;

fn main() {
    #[cfg(not(all(target_os = "linux", feature = "wayland")))]
    panic!("This example requires the `wayland` feature and a linux system.");

    #[cfg(all(target_os = "linux", feature = "wayland"))]
    let app = Application::new().with_assets(Assets {});

    app.run(|cx| {
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

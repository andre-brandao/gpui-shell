use gpui::Application;

mod bar;
mod services;
mod widgets;

fn main() {
    #[cfg(not(all(target_os = "linux", feature = "wayland")))]
    panic!("This example requires the `wayland` feature and a linux system.");

    #[cfg(all(target_os = "linux", feature = "wayland"))]
    Application::new().run(|cx| {
        // Create all services once at startup
        let services = services::Services::new(cx);

        // Open the bar window with shared services
        bar::open(services.clone(), cx);

        // You can open additional windows sharing the same services:
        // bar::open(services.clone(), cx);  // Another bar (e.g., different monitor)
        // launcher::open(services.clone(), cx);
    });
}

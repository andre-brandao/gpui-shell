use gpui::Application;

mod bar;
mod services;
mod widgets;

fn main() {
    #[cfg(not(all(target_os = "linux", feature = "wayland")))]
    panic!("This example requires the `wayland` feature and a linux system.");

    #[cfg(all(target_os = "linux", feature = "wayland"))]
    Application::new().run(|cx| {
        // Open the bar window
        bar::open(cx);

        // You can open additional independent windows here, e.g.:
        // launcher::open(cx);
    });
}

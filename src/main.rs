use gpui::{AppContext, Application};

mod bar;
mod services;
mod widgets;

fn main() {
    #[cfg(not(all(target_os = "linux", feature = "wayland")))]
    panic!("This example requires the `wayland` feature and a linux system.");

    #[cfg(all(target_os = "linux", feature = "wayland"))]
    Application::new().run(|cx| {
        use crate::services::compositor::Compositor;

        let compositor = cx.new(Compositor::new);
        // Open the bar window
        // bar::open(cx);
        bar::open_with_compositor(compositor.clone(), cx);
        // You can open additional independent windows here, e.g.:
        // launcher::open(cx);
    });
}

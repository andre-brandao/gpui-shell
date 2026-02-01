mod bar;
mod widgets;

fn main() {
    // #[cfg(all(target_os = "linux", feature = "wayland"))]
    bar::init();

    // tab::init();
    // paint::init();
    // popover::init();
    // scroll::init();
    // menu::init();
    // shadow::init();
    // tree::init();
    //
    // examples::win_shadow::init();

    #[cfg(not(all(target_os = "linux", feature = "wayland")))]
    panic!("This example requires the `wayland` feature and a linux system.");
}

mod bar;

fn main() {
    #[cfg(all(target_os = "linux", feature = "wayland"))]
    bar::init();

    #[cfg(not(all(target_os = "linux", feature = "wayland")))]
    panic!("This example requires the `wayland` feature and a linux system.");
}

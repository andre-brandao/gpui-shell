{
  lib,
  mkShell,

  nixfmt-rfc-style,
  nixd,

  llvmPackages,
  glibc,

  cmake,
  freetype,
  fontconfig,
  libpulseaudio,
  libxkbcommon,
  openssl,
  pkg-config,
  rustToolchain,
  vulkan-loader,
  wayland,
  xorg,
  glib,
  atk,
  gtk3,
  cairo,
  pango,
  gdk-pixbuf,
  pipewire,
  systemd,
}:

mkShell rec {
  packages = [
    nixd
    nixfmt-rfc-style

    rustToolchain
  ];

  nativeBuildInputs = [
    pkg-config
    llvmPackages.libclang
    cmake
  ];

  buildInputs = [
    openssl
    fontconfig
    libxkbcommon
    xorg.libxcb
    xorg.libX11
    wayland
    vulkan-loader
    freetype
    libpulseaudio
    # --

    glib
    atk
    gtk3
    cairo
    pango
    gdk-pixbuf
    pipewire
    systemd
  ];

  env = {
    LD_LIBRARY_PATH = "${lib.makeLibraryPath buildInputs}";
    RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
    LIBCLANG_PATH = "${llvmPackages.libclang.lib}/lib";
    BINDGEN_EXTRA_CLANG_ARGS = "-isystem ${llvmPackages.libclang.lib}/lib/clang/${lib.versions.major llvmPackages.libclang.version}/include -isystem ${glibc.dev}/include";
  };
}

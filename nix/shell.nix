{
  lib,
  mkShell,

  nixfmt-rfc-style,
  nixd,

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
}:

mkShell rec {
  packages = [
    nixd
    nixfmt-rfc-style

    rustToolchain
  ];

  nativeBuildInputs = [
    pkg-config
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
  ];

  env = {
    LD_LIBRARY_PATH = "${lib.makeLibraryPath buildInputs}";
    RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
  };
}

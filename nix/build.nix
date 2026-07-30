{
  lib,

  crane,
  rustPlatform,
  rustToolchain,

  makeWrapper,
  matugen,

  freetype,
  fontconfig,
  libpulseaudio,
  libxkbcommon,
  openssl,
  pkg-config,
  vulkan-loader,
  wayland,
  xorg,

  systemd,
  pipewire,
  # glib,
  # pango,
  # gdk-pixbuf,
  # atk,
  # cairo,
  # gtk3,

  profile ? "release",
}:
let
  pname = "gpuishell";
  mkIncludeFilter =
    root': path: type:
    let
      root = toString root' + "/";
      relPath = lib.removePrefix root path;
      topLevelInclueds = [
        "crates"
        "assets"
        "Cargo.toml"
      ];
      firstComp = builtins.head (lib.path.subpath.components relPath);
    in
    builtins.elem firstComp topLevelInclueds;

  craneLib = crane.overrideToolchain rustToolchain;
  commonArgs =
    let
      mgsCargoLock = builtins.fromTOML (builtins.readFile ../crates/app/Cargo.toml);
    in
    rec {
      inherit pname;
      version = mgsCargoLock.package.version;
      src = builtins.path {
        path = ../.;
        filter = mkIncludeFilter ../.;
        name = "source";
      };

      cargoLock = ../Cargo.lock;

      nativeBuildInputs = [
        pkg-config
        rustPlatform.bindgenHook
        makeWrapper
        matugen
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
        pipewire
        systemd
        # glib
        # atk
        # gtk3
        # cairo
        # pango
        # gdk-pixbuf
      ];

      stdenv =
        pkgs:
        let
          base = pkgs.llvmPackages.stdenv;
          addBinTools = old: {
            cc = old.cc.override {
              inherit (pkgs.llvmPackages) bintools;
            };
          };
          custom = lib.pipe base [
            (stdenv: stdenv.override addBinTools)
            pkgs.stdenvAdapters.useMoldLinker
          ];
        in
        custom;

      env = {
        CARGO_PROFILE = profile;
        TARGET_DIR = "target/" + (if profile == "dev" then "debug" else profile);
        NIX_LDFLAGS = "-rpath ${
          lib.makeLibraryPath [
            vulkan-loader
            wayland
          ]
        }";
      };

      dontPatchELF = true;

      doCheck = false;

      cargoVendorDir = craneLib.vendorCargoDeps {
        inherit src cargoLock;

        # bindgen's clang_macro_fallback drops its scratch .macro_eval.c and
        # *.pch into the build script's CWD, which is the vendored crate dir -
        # read-only in the store. The write fails, the fallback silently turns
        # itself off, and every cast macro (SPA_ID_INVALID, ...) disappears from
        # the bindings. Point the scratch dir at OUT_DIR instead.
        overrideVendorCargoPackage =
          p: drv:
          if p.name == "libspa-sys" || p.name == "pipewire-sys" then
            drv.overrideAttrs (_: {
              postPatch = ''
                substituteInPlace build.rs \
                  --replace-fail '.clang_macro_fallback()' \
                    '.clang_macro_fallback().clang_macro_fallback_build_dir(env::var("OUT_DIR").unwrap())'
              '';
            })
          else
            drv;
      };

    };
  cargoArtifacts = craneLib.buildDepsOnly commonArgs;

in
craneLib.buildPackage (
  lib.recursiveUpdate commonArgs {
    inherit cargoArtifacts;

    meta = {
      mainProgram = pname;
    };
  }
)

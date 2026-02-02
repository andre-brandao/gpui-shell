{
  lib,

  crane,
  rustPlatform,
  rustToolchain,

  makeWrapper,

  freetype,
  fontconfig,
  libpulseaudio,
  libxkbcommon,
  openssl,
  pkg-config,
  vulkan-loader,
  wayland,
  xorg,

  profile ? "release",
}:
let
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
      pname = "mgs";
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
      };

    };
  cargoArtifacts = craneLib.buildDepsOnly commonArgs;

in
craneLib.buildPackage (
  lib.recursiveUpdate commonArgs {
    inherit cargoArtifacts;
  }
)

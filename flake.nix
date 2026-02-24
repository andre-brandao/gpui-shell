{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    crane.url = "github:ipetkov/crane";

    matugen = {
      url = "github:/InioX/Matugen";
    };

  };

  outputs =
    {
      self,
      crane,
      nixpkgs,
      rust-overlay,
      ...
    }@inputs:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

      forAllSystems =
        f:
        nixpkgs.lib.genAttrs supportedSystems (
          system:
          let
            pkgs = import nixpkgs {
              inherit system;
              overlays = [ self.overlays.default ];
            };
          in
          f pkgs
        );

      mkBuild =
        pkgs:
        let
          rustBin = rust-overlay.lib.mkRustBin { } pkgs;
        in
        pkgs.callPackage ./nix/build.nix {
          crane = crane.mkLib pkgs;
          rustToolchain = rustBin.fromRustupToolchainFile ./rust-toolchain.toml;
        };

      mkDevShell =
        pkgs:
        let
          rustBin = rust-overlay.lib.mkRustBin { } pkgs;
        in
        pkgs.callPackage ./nix/shell.nix {
          rustToolchain = rustBin.fromRustupToolchainFile ./rust-toolchain.toml;
        };

    in
    {

      packages = forAllSystems (pkgs: rec {
        default = pkgs.gpuishell;
        debug = default.override { profile = "dev"; };
      });

      devShells = forAllSystems (pkgs: {
        default = mkDevShell pkgs;
      });

      apps = forAllSystems (pkgs: {
        default = {
          type = "app";
          program = "${pkgs.lib.getExe pkgs.gpuishell}";
        };
      });

      overlays.default = final: prev: {
        gpuishell = mkBuild final;
        matugen = inputs.matugen.packages.${final.system}.default;
      };
    };
}

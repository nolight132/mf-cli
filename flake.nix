{
  description = "CLI tool and seamless kernel module for Arturia MiniFuse 1/2";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      packages = forAllSystems (pkgs: {
        default = self.packages.${pkgs.stdenv.hostPlatform.system}.mf-cli;
        mf-cli = pkgs.callPackage ./nix/package.nix { };
        minifuse-mod = pkgs.linuxPackages.callPackage ./nix/kernel-module.nix { };
      });

      overlays.default = final: _prev: {
        mf-cli = final.callPackage ./nix/package.nix { };
      };

      nixosModules.default = ./nix/module.nix;

      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          packages = [
            pkgs.cargo
            pkgs.rustc
            pkgs.clippy
            pkgs.rustfmt
            pkgs.rust-analyzer
            pkgs.pkg-config
            pkgs.libusb1
          ];
        };
      });

      formatter = forAllSystems (pkgs: pkgs.nixfmt-tree);
    };
}

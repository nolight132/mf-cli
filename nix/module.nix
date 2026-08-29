{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.hardware.minifuse;
in
{
  options.hardware.minifuse = {
    enable = lib.mkEnableOption "control tooling for the Arturia MiniFuse 1/2";

    package = lib.mkOption {
      type = lib.types.package;
      default = pkgs.callPackage ./package.nix { };
      defaultText = lib.literalExpression "pkgs.callPackage ./package.nix { }";
      description = "The mf-cli package to install.";
    };

    kernelModule = {
      enable = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = ''
          Build and load the minifuse_mod kernel module, which lets mf-cli
          toggle the device without interrupting the audio stream. With this
          off, mf-cli falls back to userspace USB control.
        '';
      };

      package = lib.mkOption {
        type = lib.types.package;
        default = config.boot.kernelPackages.callPackage ./kernel-module.nix { };
        defaultText = lib.literalExpression "config.boot.kernelPackages.callPackage ./kernel-module.nix { }";
        description = "The minifuse_mod kernel module package to build.";
      };
    };
  };

  config = lib.mkIf cfg.enable {
    environment.systemPackages = [ cfg.package ];
    services.udev.packages = [ cfg.package ];

    boot.extraModulePackages = lib.mkIf cfg.kernelModule.enable [ cfg.kernelModule.package ];
    boot.kernelModules = lib.mkIf cfg.kernelModule.enable [ "minifuse_mod" ];
  };
}

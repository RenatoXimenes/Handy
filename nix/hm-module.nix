# Home-manager module for Vozel speech-to-text
#
# Provides a systemd user service for autostart.
# Usage: imports = [ vozel.homeManagerModules.default ];
#        services.vozel.enable = true;
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.services.vozel;
in
{
  options.services.vozel = {
    enable = lib.mkEnableOption "Vozel speech-to-text user service";

    package = lib.mkOption {
      type = lib.types.package;
      defaultText = lib.literalExpression "vozel.packages.\${system}.vozel";
      description = "The Vozel package to use.";
    };
  };

  config = lib.mkIf cfg.enable {
    systemd.user.services.vozel = {
      Unit = {
        Description = "Vozel speech-to-text";
        After = [ "graphical-session.target" ];
        PartOf = [ "graphical-session.target" ];
      };
      Service = {
        ExecStart = "${cfg.package}/bin/vozel";
        Restart = "on-failure";
        RestartSec = 5;
      };
      Install.WantedBy = [ "graphical-session.target" ];
    };
  };
}

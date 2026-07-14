{
  self ? null,
  config,
  pkgs,
  lib,
  ...
}:

let
  flakePackages = if self != null && self ? packages then self.packages else { };
  hasFlakeServer =
    builtins.hasAttr pkgs.system flakePackages
    && builtins.hasAttr "server" flakePackages.${pkgs.system};
  hasFlakeCombo =
    builtins.hasAttr pkgs.system flakePackages
    && builtins.hasAttr "service-tui" flakePackages.${pkgs.system};
  serverPkg = if hasFlakeServer then flakePackages.${pkgs.system}.server else null;
  comboPkg = if hasFlakeCombo then flakePackages.${pkgs.system}.service-tui else null;
  execPath =
    if config.services.bldhnd.package != null then config.services.bldhnd.package else serverPkg;
  useCombo =
    config.services.bldhnd.mode == "combo"
    || (config.services.bldhnd.package == null && comboPkg != null);
  effectivePkg =
    if config.services.bldhnd.package != null then config.services.bldhnd.package
    else if useCombo then comboPkg
    else serverPkg;
in
{
  options = {
    services.bldhnd = {
      enable = lib.mkEnableOption "bldhnd server";
      package = lib.mkOption {
        type = lib.types.nullOr lib.types.package;
        default = null;
        description = "Package to run for the server. If null the flake-built server package is used.";
      };
      mode = lib.mkOption {
        type = lib.types.enum [
          "server"
          "combo"
        ];
        default = "server";
        description = "Whether to run just the server or the service+TUI combo.";
      };
      user = lib.mkOption {
        type = lib.types.str;
        default = "root";
        description = "User to run the service as.";
      };
    };
  };

  config = lib.mkIf config.services.bldhnd.enable {
    assertions = [
      {
        assertion = effectivePkg != null;
        message = "services.bldhnd.package must be set when the flake-built server package is unavailable for ${pkgs.system}.";
      }
    ];

    systemd.services.bldhnd-server = {
      description = "bldhnd server";
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        ExecStart =
          if useCombo then
            "${pkgs.bash}/bin/bash -c 'cd /var/lib/bldhnd && ${effectivePkg}/bin/bh-service & ${effectivePkg}/bin/bldhnd'"
          else
            "${execPath}/bin/bh-server";
        Restart = "on-failure";
        User = config.services.bldhnd.user;
        StateDirectory = "bldhnd";
        Environment = "BLDHND_DIR=/var/lib/bldhnd";
      };
    };
  };
}

{ self ? null, config, pkgs, lib, ... }:

let
  serverPkg =
    if self != null && self ? packages && builtins.hasAttr pkgs.system self.packages && builtins.hasAttr "server" self.packages.${pkgs.system}
    then self.packages.${pkgs.system}.server
    else null;
  execPath = if config.services.bldhnd.package != null then config.services.bldhnd.package else serverPkg;
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
        assertion = execPath != null;
        message = "services.bldhnd.package must be set when the flake-built server package is unavailable for ${pkgs.system}.";
      }
    ];

    systemd.services.bldhnd-server = {
      description = "bldhnd server";
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        ExecStart = "${execPath}/bin/bh-server";
        Restart = "on-failure";
        User = config.services.bldhnd.user;
        StateDirectory = "bldhnd";
        Environment = "BLDHND_DIR=/var/lib/bldhnd";
      };
    };
  };
}

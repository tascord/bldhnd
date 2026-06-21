{ config, pkgs, lib, ... }:

let
  serverPkg = pkgs.callPackage ../server {};
  execPath = if config.services.bldhnd.package != null then config.services.bldhnd.package else serverPkg;
in
{
  options = {
    services.bldhnd = {
      enable = lib.mkEnableOption "bldhnd server";
      package = lib.mkOption {
        type = lib.types.nullOr lib.types.path;
        default = null;
        description = "Path to a prebuilt package to run for the server. If null the flake-built server package is used.";
      };
      user = lib.mkOption {
        type = lib.types.str;
        default = "root";
        description = "User to run the service as.";
      };
    };
  };

  config = lib.mkIf config.services.bldhnd.enable {
    systemd.services.bldhnd-server = {
      description = "bldhnd server";
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        ExecStart = "${execPath}/bin/server";
        Restart = "on-failure";
        User = config.services.bldhnd.user;
        # Use systemd's StateDirectory to create/own /var/lib/bldhnd and make
        # it available to the service. Also export BLDHND_DIR so the server
        # uses the system path instead of ~/.bldhnd on NixOS.
        StateDirectory = "bldhnd";
        Environment = "BLDHND_DIR=/var/lib/bldhnd";
      };
    };
  };
}

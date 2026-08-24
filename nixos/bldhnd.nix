{
  self ? null,
  config,
  pkgs,
  lib,
  ...
}:

{
  options = {
    services.bldhnd-server = {
      enable = lib.mkEnableOption "bh-server";
      package = lib.mkOption {
        type = lib.types.nullOr lib.types.package;
        default = null;
        description = "Package to run for bh-server.";
      };
    };
    services.bldhnd-service = {
      enable = lib.mkEnableOption "bh-service";
      package = lib.mkOption {
        type = lib.types.nullOr lib.types.package;
        default = null;
        description = "Package to run for bh-service.";
      };
    };
    programs.bldhnd = {
      enable = lib.mkEnableOption "bldhnd TUI";
      package = lib.mkOption {
        type = lib.types.nullOr lib.types.package;
        default = null;
        description = "Package to run for bldhnd TUI.";
      };
    };
  };

  config = lib.mkIf (config.services.bldhnd-server.enable || config.services.bldhnd-service.enable || config.programs.bldhnd.enable) {
    assertions = [
      {
        assertion = config.services.bldhnd-server.package != null;
        message = "services.bldhnd-server.package must be set";
      }
      {
        assertion = config.services.bldhnd-service.package != null;
        message = "services.bldhnd-service.package must be set";
      }
      {
        assertion = config.programs.bldhnd.package != null;
        message = "programs.bldhnd.package must be set";
      }
    ];

    systemd.services.bldhnd-server = lib.mkIf config.services.bldhnd-server.enable {
      description = "bldhnd server";
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        ExecStart = "${config.services.bldhnd-server.package}/bin/bh-server";
        Restart = "on-failure";
        StateDirectory = "bldhnd";
        Environment = "BLDHND_DIR=/var/lib/bldhnd";
      };
    };

    systemd.services.bldhnd-service = lib.mkIf config.services.bldhnd-service.enable {
      description = "bldhnd service";
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        ExecStart = "${config.services.bldhnd-service.package}/bin/bh-service";
        Restart = "on-failure";
        StateDirectory = "bldhnd";
        Environment = "BLDHND_DIR=/var/lib/bldhnd";
      };
    };
  };
}
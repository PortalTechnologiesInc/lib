{
  config,
  lib,
  pkgs,
  ...
}:

with lib;

let
  cfg = config.services.portal-backend;
in
{
  options.services.portal-backend = {
    enable = mkEnableOption "Portal backend";

    package = mkOption {
      type = types.package;
      default = pkgs.portal-backend;
      description = "The portal-backend package to use.";
    };

    authToken = mkOption {
      type = types.str;
      description = "Auth token to use for the backend";
    };

    stateDir = mkOption {
      type = types.str;
      default = "/var/lib/portal-backend";
      description = "State directory for portal-backend";
    };

    user = mkOption {
      type = types.str;
      default = "portal-backend";
      description = "User account under which portal-backend runs";
    };

    group = mkOption {
      type = types.str;
      default = "portal-backend";
      description = "Group account under which portal-backend runs";
    };
  };

  config =
    let
      # Combine all environment variables
      envConfig = {
        DATABASE_PATH = "${cfg.stateDir}/db.sqlite";
        AUTH_TOKEN = cfg.authToken;
      };
    in
    mkIf cfg.enable {
      systemd.services.portal-backend = {
        description = "Portal backend";
        wantedBy = [ "multi-user.target" ];
        after = [ "portal-rest.service" ];
        requires = [ "portal-rest.service" ];

        environment = envConfig;

        serviceConfig = {
          ExecStart = "${lib.getExe cfg.package}";
          Restart = "always";
          ProtectSystem = "strict";
          ProtectHome = true;
          PrivateTmp = true;
          NoNewPrivileges = true;
          StateDirectory = "portal-backend";
          User = cfg.user;
          Group = cfg.group;
        };
      };

      systemd.tmpfiles.rules = [
        "d ${cfg.stateDir}                            0700 ${cfg.user} ${cfg.group} - -"
      ];

      services.portal-rest = {
        enable = true;
        authToken = cfg.authToken;
      };

      users.users.${cfg.user} = {
        isSystemUser = true;
        group = cfg.group;
      };
      users.groups.${cfg.group} = {};
    };
}
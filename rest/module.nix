{
  config,
  lib,
  pkgs,
  ...
}:

with lib;

let
  cfg = config.services.portal-rest;
in
{
  options.services.portal-rest = {
    enable = mkEnableOption "Portal rest api";

    package = mkOption {
      type = types.package;
      default = pkgs.portal-rest;
      description = "The portal-rest package to use.";
    };

    authToken = mkOption {
      type = types.str;
      description = "Auth token to use authenticate clients";
    };

    nostrKey = mkOption {
      type = types.str;
      description = "Nostr private key in nsec format";
    };

    nostrRelays = mkOption {
      type = types.listOf types.str;
      default = [ "wss://relay.nostr.net" "wss://relay.damus.io" ];
      description = "List of Nostr relay URLs";
    };

    nwcUrl = mkOption {
      type = types.nullOr types.str;
      default = null;
      description = "Nostr Wallet Connect URL (optional)";
    };

    nostrSubkeyProof = mkOption {
      type = types.nullOr types.str;
      default = null;
      description = "Nostr subkey proof JSON (optional)";
    };

    user = mkOption {
      type = types.str;
      default = "portal-rest";
      description = "User account under which portal-rest runs";
    };

    group = mkOption {
      type = types.str;
      default = "portal-rest";
      description = "Group under which portal-rest runs";
    };

    rustLog = mkOption {
      type = types.str;
      default = "info";
      description = "Rust log level";
    };
  };

  config =
    let
      # Combine all environment variables
      envConfig = {
        AUTH_TOKEN = cfg.authToken;
        NOSTR_KEY = cfg.nostrKey;
        NOSTR_RELAYS = lib.concatStringsSep "," cfg.nostrRelays;
        RUST_LOG = cfg.rustLog;
      } // lib.optionalAttrs (cfg.nwcUrl != null) {
        NWC_URL = cfg.nwcUrl;
      } // lib.optionalAttrs (cfg.nostrSubkeyProof != null) {
        NOSTR_SUBKEY_PROOF = cfg.nostrSubkeyProof;
      };
    in
    mkIf cfg.enable {
      systemd.services.portal-rest = {
        description = "Portal rest api";
        wantedBy = [ "multi-user.target" ];
        wants = [ "network-online.target" ];
        after = [ "network-online.target" ];

        environment = envConfig;

        serviceConfig = {
          ExecStart = "${lib.getExe cfg.package}";
          Restart = "always";
          ProtectSystem = "strict";
          ProtectHome = true;
          PrivateTmp = true;
          NoNewPrivileges = true;
          StateDirectory = "portal-rest";
          User = cfg.user;
          Group = cfg.group;
        };
      };

      users.users.${cfg.user} = {
        isSystemUser = true;
        group = cfg.group;
      };
      users.groups.${cfg.group} = {};
    };
}
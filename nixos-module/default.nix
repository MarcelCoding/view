{ pkgs, config, lib, ... }:
let
  cfg = config.TLMS.view;
in
{
  options.TLMS.view = with lib; {
    enable = mkOption {
      type = types.bool;
      default = false;
      description = ''Wether to enable the view service'';
    };
    http = {
      api = {
        host = mkOption {
          type = types.str;
          default = "127.0.0.1";
          description = ''
            To which IP view should bind.
          '';
        };
        port = mkOption {
          type = types.port;
          default = 8080;
          description = ''
            To which port should view bind.
          '';
        };
      };
      files = {
        host = mkOption {
          type = types.str;
          default = "127.0.0.1";
          description = ''
            To which IP view should bind.
          '';
        };
        port = mkOption {
          type = types.port;
          default = 8080;
          description = ''
            To which port should view bind.
          '';
        };
      };
    };
    database = {
      host = mkOption {
        type = types.str;
        default = "127.0.0.1";
        description = ''
          Database host
        '';
      };
      port = mkOption {
        type = types.port;
        default = 5453;
        description = ''
          Database port
        '';
      };
      user = mkOption {
        type = types.str;
        default = "tlms";
        description = ''
          Database User to connect as
        '';
      };
      passwordFile = mkOption {
        type = types.either types.path types.string;
        default = "";
        description = ''password file from which the postgres password can be read'';
      };
      database = mkOption {
        type = types.str;
        default = "tlms";
        description = ''
          Database which should be used
        '';
      };
    };
    tokenPath = mkOption {
      type = types.str;
      description = ''token from which content will be accepted and served.'';
    };
    rootDir = mkOption {
      type = types.str;
      default = "/var/lib/view/";
      description = ''where the content will go that should be served.'';
    };
    user = mkOption {
      type = types.str;
      default = "view";
      description = ''systemd user'';
    };
    group = mkOption {
      type = types.str;
      default = "view";
      description = ''group of systemd user'';
    };
    logLevel = mkOption {
      type = types.str;
      default = "info";
      description = ''log level of the application'';
    };
  };

  config = lib.mkIf cfg.enable {
    users.groups.TLMS-radio = {
      name = "TLMS-radio";
      members = [
        "wartrammer"
        "data-accumulator"
        "view"
      ];
      gid = 1501;
    };

    systemd = {
      services = {
        "view" = {
          enable = true;
          wantedBy = [ "multi-user.target" ];
          after = ["postgresql.service" "view-setup.service"];

          script = ''
              exec ${pkgs.view}/bin/view&
          '';

          environment = {
            "RUST_LOG" = "${cfg.logLevel}";
            "RUST_BACKTRACE" = if (cfg.logLevel == "info") then "0" else "1";
            "VIEW_DB_URL" = "${cfg.database.host}:${toString cfg.database.port}/${cfg.database.database}";
            "VIEW_USER" = "${cfg.database.user}";
            "VIEW_PASS_PATH" = "${cfg.database.passwordFile}";
            "VIEW_ROOT_DIR" = "${cfg.rootDir}";
            "VIEW_SERVE_ADDR" = "${cfg.http.files.host}:${toString cfg.http.files.port}";
            "VIEW_MGNT_ADDR" = "${cfg.http.api.host}:${toString cfg.http.api.port}";
            "VIEW_MGNT_TOKEN_PATH" = "${cfg.tokenPath}";
          };

          serviceConfig = {
            Type = "forking";
            User = cfg.user;
            Restart = "always";
          };
        };
        "view-setup" = {
          enable = true;
          description = "Prepare HedgeDoc postgres database";
          wantedBy = [ "multi-user.target" ];
          after = [ "networking.target" "postgresql.service" ];
          serviceConfig.Type = "oneshot";

          path = [ pkgs.sudo config.services.postgresql.package ];
          script = ''
            # setup directory
            mkdir -p ${cfg.rootDir}
            chown ${cfg.user} ${cfg.rootDir}

            # setup postgres postgres user with password
            # TODO: add where the postgres instance is running
            sudo -u ${config.services.postgresql.superUser} psql -c "ALTER ROLE ${cfg.database.user} WITH PASSWORD '$(cat ${cfg.database.passwordFile})'"
          '';
        };
      };
    };

    # user accounts for systemd units
    users.users."${cfg.user}" = {
      name = "${cfg.user}";
      description = "This guy runs view";
      isNormalUser = false;
      isSystemUser = true;
      group = cfg.group;
      uid = 1503;
      extraGroups = [ ];
    };
  };
}

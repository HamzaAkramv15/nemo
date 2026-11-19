# Home Manager: programs.nemo. The home half of what install.sh does. The shell edits its own files
# at run time (settings, keys, screens, portals), so those are seeded once and stay writable; only
# files the shell never writes are links. Based on the module TheCodedKid wrote for #1.
self:
{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.programs.nemo;
  run = import ./runtime.nix pkgs;
  cfgDir = config.xdg.configHome;
  share = "${cfg.package}/share/nemo";
  json = pkgs.formats.json { };
  mimeOf =
    app:
    let
      line = lib.findFirst (lib.hasPrefix "MimeType=") "MimeType=" (
        lib.splitString "\n" (builtins.readFile "${self.inputs.${app}}/config/${app}.desktop")
      );
    in
    lib.filter (m: m != "") (lib.splitString ";" (lib.removePrefix "MimeType=" line));
  defaults = lib.genAttrs [ "nemo-files" "nemo-view" "nemo-play" ] (app: "${app}.desktop");
in
{
  options.programs.nemo = {
    enable = lib.mkEnableOption "nemo, the pixel-art shell for niri";

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.stdenv.hostPlatform.system}.nemo;
      defaultText = lib.literalExpression "nemo.packages.\${system}.nemo";
      description = "The nemo package (built with updates handed to Nix).";
    };

    settings = lib.mkOption {
      inherit (json) type;
      default = { };
      example = lib.literalExpression ''
        {
          palette = "sakura";
          lang = "en";
          light = false;
        }
      '';
      description = ''
        Keys of {file}`~/.config/nemo/shell.json` (camelCase, see docs/CONFIG.md). Merged into the
        file on every activation: these keys win, everything else set in the settings panel stays.
      '';
    };

    niriConfig = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = ''
        Connect nemo to niri's {file}`~/.config/niri/config.kdl`, like install.sh: one include line
        at its top brings nemo's part ({file}`~/.config/nemo/niri/nemo.kdl`: the shell, its keys,
        window rules, colours and niri look), and what you write below it wins. Without a config
        nemo's starting one is written. A config.kdl that is a link into the Nix store (niri's
        config made by home-manager) is left alone: add
        `include optional=true "~/.config/nemo/niri/nemo.kdl"` to it yourself.
      '';
    };

    kitty = lib.mkOption {
      type = lib.types.bool;
      default = config.programs.kitty.enable;
      defaultText = lib.literalExpression "config.programs.kitty.enable";
      description = "Link nemo's kitty look and include it from programs.kitty.";
    };

    defaultApplications = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = ''
        nemo-files for folders, nemo-view for pictures and nemo-play for video and music in
        xdg.mimeApps (takes effect with `xdg.mimeApps.enable`).
      '';
    };

    extraPackages = lib.mkOption {
      type = lib.types.listOf lib.types.package;
      default = run.base ++ run.apps ++ run.tools;
      defaultText = lib.literalMD "the commands the shell calls (`nemo doctor` lists them)";
      description = ''
        Programs the shell runs. On NixOS the system module installs them too; system services
        (NetworkManager, gvfs, udisks, polkit, greetd) belong there.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    home.packages = [ cfg.package ] ++ cfg.extraPackages;

    fonts.fontconfig.enable = true;
    xdg.configFile."fontconfig/conf.d/50-nemo-pixel.conf".source = "${share}/50-nemo-pixel.conf";

    # the file dialog and screen-share portal and «show in folder», where install.sh puts them
    xdg.dataFile = {
      "xdg-desktop-portal/portals/nemo.portal".source =
        "${cfg.package}/share/xdg-desktop-portal/portals/nemo.portal";
      "dbus-1/services/org.freedesktop.impl.portal.desktop.nemo.service".source =
        "${cfg.package}/share/dbus-1/services/org.freedesktop.impl.portal.desktop.nemo.service";
      "dbus-1/services/org.freedesktop.FileManager1.service".source =
        "${cfg.package}/share/dbus-1/services/org.freedesktop.FileManager1.service";
      "xdg-desktop-portal/portals/nemo-agents.portal".source =
        "${cfg.package}/share/xdg-desktop-portal/portals/nemo-agents.portal";
      "dbus-1/services/org.freedesktop.impl.portal.desktop.nemoagents.service".source =
        "${cfg.package}/share/dbus-1/services/org.freedesktop.impl.portal.desktop.nemoagents.service";
    };

    xdg.configFile."kitty/nemo.conf" = lib.mkIf cfg.kitty { source = "${share}/nemo.conf"; };
    programs.kitty.extraConfig = lib.mkIf cfg.kitty (lib.mkAfter "include nemo.conf");

    xdg.mimeApps.defaultApplications = lib.mkIf cfg.defaultApplications (
      {
        "inode/directory" = defaults.nemo-files;
      }
      // lib.genAttrs (mimeOf "nemo-view") (_: defaults.nemo-view)
      // lib.genAttrs (mimeOf "nemo-play") (_: defaults.nemo-play)
    );

    home.activation.nemo = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
      nemo_cfg=${lib.escapeShellArg cfgDir}

      # settings: declared keys over what the panel saved
      ${lib.optionalString (cfg.settings != { }) ''
        nemo_json="$nemo_cfg/nemo/shell.json"
        run mkdir -p "$nemo_cfg/nemo"
        # before nemo 3.0 the file lay in ~/.config; the shell moves it too, this only comes first
        if [[ -f "$nemo_cfg/nemo-shell.json" && ! -e "$nemo_json" ]]; then
          run mv "$nemo_cfg/nemo-shell.json" "$nemo_json"
        fi
        if [[ ! -e "$nemo_json" && ! -v DRY_RUN ]]; then
          echo '{}' >"$nemo_json"
        fi
        if [[ -e "$nemo_json" ]] && ! ${pkgs.jq}/bin/jq -e 'type == "object"' "$nemo_json" >/dev/null 2>&1; then
          warnEcho "nemo: $nemo_json is not a JSON object, programs.nemo.settings not applied"
        elif [[ ! -v DRY_RUN ]]; then
          ${pkgs.jq}/bin/jq --indent 2 -s '.[0] * .[1]' "$nemo_json" ${json.generate "nemo-shell-settings.json" cfg.settings} >"$nemo_json.tmp" \
            && mv -f "$nemo_json.tmp" "$nemo_json"
        fi
      ''}

      # portals: the shell rewrites this file (settings → services), so a copy, once
      if [[ ! -e "$nemo_cfg/xdg-desktop-portal/niri-portals.conf" ]]; then
        run ${pkgs.coreutils}/bin/install -Dm644 ${share}/niri-portals.conf "$nemo_cfg/xdg-desktop-portal/niri-portals.conf"
      fi

      ${lib.optionalString cfg.niriConfig ''
        if [[ ! -v DRY_RUN ]]; then
          ${cfg.package}/bin/nemo-shell niri connect || warnEcho "nemo: niri's config.kdl was left as it is (nemo niri status)"
        fi
      ''}

      # nemo's part of niri (keys, window rules, its look) before the first login; the shell keeps it current
      if [[ ! -v DRY_RUN ]]; then
        ${cfg.package}/bin/nemo-shell niri write || true
      fi
    '';
  };
}

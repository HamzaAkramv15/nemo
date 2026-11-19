{
  description = "nemo: a lightweight pixel-art Wayland desktop for the niri compositor";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    # the modules live in their own repositories; the versions are those of modules.lock (the shell's
    # services, nemo-agents, and the drawing library nemo-px are part of this repository)
    nemo-files = {
      url = "github:HamzaAkramv15/nemo-files/nemo-v0.2.2";
      flake = false;
    };
    nemo-view = {
      url = "github:HamzaAkramv15/nemo-view/nemo-v0.2.2";
      flake = false;
    };
    nemo-play = {
      url = "github:HamzaAkramv15/nemo-play/nemo-v0.2.2";
      flake = false;
    };
    nemo-shot = {
      url = "github:HamzaAkramv15/nemo-shot/nemo-v0.3.2";
      flake = false;
    };
    nemo-greet = {
      url = "github:HamzaAkramv15/nemo-greet/nemo-v0.2.0";
      flake = false;
    };
  };

  outputs =
    { self, nixpkgs, ... }@inputs:
    let
      inherit (nixpkgs) lib;
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAll = f: lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
      version = lib.fileContents ./VERSION;
      # only what the build reads: a change to docs, CI or the installer does not rebuild nemo. keys/: the
      # release keys are compiled into the shell (update.rs), so a check never trusts a file it is handed
      src = lib.fileset.toSource {
        root = ./.;
        fileset = lib.fileset.unions [
          ./LICENSE
          ./VERSION
          ./config
          ./fonts
          ./keys
          (lib.fileset.difference ./shell (lib.fileset.maybeMissing ./shell/target))
        ];
      };
      modules = lib.getAttrs [
        "nemo-files"
        "nemo-view"
        "nemo-play"
        "nemo-shot"
        "nemo-greet"
      ] inputs;
      packagesFor = pkgs: {
        nemo = pkgs.callPackage ./nix/package.nix { inherit version src modules; };
      };
    in
    {
      packages = forAll (
        pkgs:
        packagesFor pkgs
        // {
          default = self.packages.${pkgs.stdenv.hostPlatform.system}.nemo;
        }
      );

      # `nix flake check`: the package builds, the NixOS module evaluates (greetd runs nemo-greet, the
      # portals, services and tools are set)
      checks = forAll (
        pkgs:
        let
          system = pkgs.stdenv.hostPlatform.system;
          c =
            (nixpkgs.lib.nixosSystem {
              modules = [
                self.nixosModules.default
                {
                  nixpkgs.hostPlatform = system;
                  programs.nemo = {
                    enable = true;
                    loginScreen.user = "nemo";
                  };
                  users.users.nemo.isNormalUser = true;
                  fileSystems."/" = {
                    device = "/dev/null";
                    fsType = "ext4";
                  };
                  boot.loader.grub.enable = false;
                  system.stateVersion = "25.11";
                }
              ];
            }).config;
        in
        {
          inherit (self.packages.${system}) nemo;
          nixos-module = pkgs.writeText "nemo-nixos-module.json" (
            builtins.toJSON {
              greeter = c.services.greetd.settings.default_session.command;
              tmpfiles = c.systemd.tmpfiles.rules;
              portals = map (p: p.name) c.xdg.portal.extraPortals;
              packages = map (p: p.name) c.environment.systemPackages;
            }
          );
        }
      );

      nixosModules.default = import ./nix/nixos-module.nix self;
      homeManagerModules.default = import ./nix/hm-module.nix self;

      overlays.default = _final: prev: packagesFor prev;

      devShells = forAll (pkgs: {
        default = pkgs.mkShell {
          inputsFrom = [ self.packages.${pkgs.stdenv.hostPlatform.system}.nemo ];
          packages = with pkgs; [
            cargo
            rustc
            clippy
            shellcheck
          ];
          # nemo-play finds libmpv at run time
          LD_LIBRARY_PATH = lib.makeLibraryPath [ pkgs.mpv-unwrapped ];
        };
      });

      formatter = forAll (pkgs: pkgs.nixfmt);
    };
}

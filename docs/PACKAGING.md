# Packaging

English · [Русский](PACKAGING.ru.md)

Notes for Nix users and distribution packagers: the ready binaries and the packages made from them, the flake, where Nemo looks for system files, and how to hand upgrades to the package manager.

## Ready binaries

Every release carries `nemo-X.Y.Z-x86_64-linux.tar.gz`: the shell and the modules linked against an old glibc, so they run on any x86_64 system with glibc 2.34 or newer, whatever the distribution. Next to it are a `.deb` per program and `SHA256SUMS`, signed by the release key (`SHA256SUMS.sig`, namespace `nemo-release`, keys in `keys/allowed_signers`). The installer takes them by itself; `tools/dist.sh` makes them (with no container: against the packages of Debian 12 unpacked into `~/.cache/nemo/sysroot`; in a Debian 12 container with `--no-sysroot`).

Inside the archive `stage.sh` puts the programs where a package wants them:

```sh
./stage.sh --prefix /usr --destdir "$pkgdir" --managed --only nemo-shell
```

`--only` takes one program (`nemo-shell`, which brings its services nemo-agents, `nemo-files` `nemo-view` `nemo-play` `nemo-shot` `nemo-greet`), without it all of them; `--managed` leaves the mark `share/nemo/package-managed`, which turns the updater off in these same binaries. A package does not carry the `org.freedesktop.FileManager1` service: it belongs to Nautilus on most systems, and «show in folder» for every app is each person's choice (`nemo-files install` asks and puts it into the home folder).

| System | File |
|---|---|
| Arch | `packaging/arch/PKGBUILD`, one package per program |
| Debian, Ubuntu | the `.deb` files of the release, `tools/dist.sh --deb` |
| Fedora, openSUSE | `packaging/fedora/nemo.spec` |
| Gentoo | `packaging/gentoo/nemo-bin-*.ebuild`, a USE flag per module |

Ubuntu 26.04 has no niri package: `install.sh` builds niri there, a `.deb` needs niri from elsewhere. A package also installs `/etc/xdg/autostart/nemo-shell.desktop`. It starts the shell (`nemo-shell autostart`) at the first login, when niri's config does not start it yet; the first-run setup opens by itself and its niri step connects Nemo with one include line. After that the config starts the shell and the autostart entry does nothing. Without systemd niri runs no autostart entries: `nemo niri connect` once from a terminal does the same.

## Nix

The flake has the package (the shell with its services nemo-agents, nemo-files, nemo-view, nemo-play, nemo-shot and the login screen nemo-greet, the modules as flake inputs at the versions of `modules.lock`), a NixOS module and a Home Manager module. The NixOS module is the system half of `install.sh`: niri, the services the shell talks to, the portals with nemo's file dialog and screen-share picker, fonts, the commands the shell runs and the login screen. The Home Manager module is the home half: niri's config, the shell's settings, kitty, fontconfig and the default apps.

```nix
{
  inputs.nemo.url = "github:HamzaAkramv15/nemo";
  inputs.nemo.inputs.nixpkgs.follows = "nixpkgs";

  # NixOS
  imports = [ nemo.nixosModules.default ];
  programs.nemo.enable = true;
  programs.nemo.loginScreen.user = "alice"; # whose shell gives the login screen its look

  # Home Manager
  imports = [ nemo.homeManagerModules.default ];
  programs.nemo.enable = true;
  programs.nemo.settings = { palette = "sakura"; lang = "en"; };
}
```

NixOS options:

| Option | Default | What |
|---|---|---|
| `programs.nemo.enable` | `false` | niri, NetworkManager, BlueZ, udisks, gvfs, polkit, power profiles, the keyring, the portals, fonts and the shell's commands |
| `programs.nemo.loginScreen.enable` | `true` | greetd starts nemo-greet inside niri; off keeps another display manager |
| `programs.nemo.loginScreen.user` | `null` | the user whose shell writes the login screen's look (palette, wallpaper, language, screens, keyboard layouts) |
| `programs.nemo.apps` | `true` | kitty, fastfetch, pavucontrol, btop, video thumbnails, bsdtar |
| `programs.nemo.tools` | `true` | night light, text and QR codes from screenshots |
| `programs.nemo.appTheme` | `true` | qt6ct and adw-gtk3 for Qt and GTK apps in the palette |

Home Manager options: `settings` (keys of `~/.config/nemo/shell.json`, merged on every activation: these win, the rest set in the settings panel stays), `niriConfig` (connects nemo to niri's `config.kdl` with one include line, `nemo niri connect`; a config that is a link into the Nix store is left alone, add `include optional=true "~/.config/nemo/niri/nemo.kdl"` to it yourself), `kitty`, `defaultApplications`, `extraPackages`.

The shell writes its own files at run time (settings, keys, screens, portals), so the modules seed them once and leave them writable. nemo-niri, the patched niri in its own repository, is not packaged: the packaged niri is the same desktop without tearing in games, the realtime main thread and the stream curtain's layer rules. `nix flake check` builds the package and evaluates the NixOS module; CI runs it on every push. The modules started from the work of TheCodedKid in #1.

## System files

The shell finds the polkit helper and the gvfs MTP backend without fixed FHS paths. On NixOS the polkit helper is the setuid wrapper `/run/wrappers/bin/polkit-agent-helper-1`, which comes first: a copy of the helper from `PATH` has no setuid bit and cannot check a password. gvfs is found through `XDG_DATA_DIRS`. The old paths under `/usr` stay as fallbacks.

The login screen: greetd runs `libexec/nemo/greet-session` as its greeter user (the installer puts it in `/usr/local/lib/nemo`). It finds `bin/nemo-greet` from its own folder (`NEMO_GREET` names another), niri in `PATH`, the sessions in the `wayland-sessions` folders of `XDG_DATA_DIRS`, and nemo-greet finds its fonts in `share/nemo-greet/fonts` next to its binary (`NEMO_GREET_FONTS`). It needs three folders: `/var/lib/nemo-greet/look` owned by the user whose shell publishes the look, `/var/lib/nemo-greet/state` (0700) and `/var/lib/nemo-greet/log` owned by the greeter user.

## Updates by the package manager

Ready binaries: `--managed` of `stage.sh` (above). From source: build with the Cargo feature `package-managed` (the flake does):

```sh
cargo build --release --locked --workspace --features nemo-shell/package-managed --manifest-path shell/Cargo.toml
```

Such a build never checks for updates on a schedule, the updates tab shows a note instead of its buttons, and `nemo update` (with any subcommand) prints «Updates are managed by your package manager.» and exits with 1. Update, release switch and rollback refuse before they touch the source clone or the installed binaries. Upgrade and roll back with the package manager. A build without the feature keeps the updater.

In the first-run setup the modules page then only says what is installed. Do not run `install.sh` over an installation that the package manager owns.

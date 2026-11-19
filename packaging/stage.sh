#!/usr/bin/env bash
# Put the ready binaries where a system-wide package wants them. For packagers: PKGBUILD, .deb, RPM,
# ebuild call it with the package's root in --destdir. A home-folder install is install.sh's job.
#   stage.sh --prefix /usr --destdir "$pkgdir" [--managed] [--only nemo-shell|nemo-files|…]
# --managed leaves the mark that hands updates and modules to the package manager (the updater stays off).
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PREFIX=/usr DESTDIR="" MANAGED=0 ONLY=""
while (($#)); do
  case $1 in
    --prefix) PREFIX="${2:?}"; shift ;;
    --destdir) DESTDIR="${2:?}"; shift ;;
    --managed) MANAGED=1 ;;
    --only) ONLY="${2:?}"; shift ;;
    *) echo "usage: stage.sh --prefix DIR [--destdir DIR] [--managed] [--only PROGRAM]" >&2; exit 2 ;;
  esac
  shift
done
D="$DESTDIR$PREFIX"
want() { [[ -z $ONLY || $ONLY == "$1" ]]; }
service() { # the source file, the program it starts
  local to="$D/share/dbus-1/services/${1##*/}"
  install -d "${to%/*}"
  sed "s|@BIN@|$PREFIX/bin/$2|" "$1" >"$to"
  chmod 644 "$to"
}
licence() { install -Dm644 "$HERE/LICENSE" "$D/share/licenses/$1/LICENSE"; }
M="$HERE/modules"
if want nemo-shell; then
  install -Dm755 "$HERE/bin/nemo-shell" "$D/bin/nemo-shell"
  ln -sfn nemo-shell "$D/bin/nemo"
  # the first login of a packaged install: niri's own config does not start the shell yet
  install -Dm644 "$HERE/config/autostart/nemo-shell.desktop" "$DESTDIR/etc/xdg/autostart/nemo-shell.desktop"
  install -Dm644 -t "$D/share/fonts/nemo" "$HERE"/fonts/*.ttf
  install -Dm644 -t "$D/share/licenses/nemo-shell" "$HERE"/fonts/OFL-*.txt
  install -Dm644 -t "$D/share/nemo" "$HERE/config/kitty/nemo.conf" "$HERE/config/fontconfig/50-nemo-pixel.conf"
  ((MANAGED)) && install -Dm644 /dev/null "$D/share/nemo/package-managed"
  licence nemo-shell
  # the shell's services (polkit window, screen sharing, automount): part of the shell's package
  install -Dm755 "$HERE/bin/nemo-agents" "$D/bin/nemo-agents"
  install -Dm644 "$M/nemo-agents/config/nemo-agents.portal" "$D/share/xdg-desktop-portal/portals/nemo-agents.portal"
  service "$M/nemo-agents/config/org.freedesktop.impl.portal.desktop.nemoagents.service" nemo-agents
fi
for m in nemo-files nemo-view nemo-play nemo-shot; do
  want "$m" || continue
  install -Dm755 "$HERE/bin/$m" "$D/bin/$m"
  licence "$m"
  [[ -f $M/$m/config/$m.desktop ]] && install -Dm644 "$M/$m/config/$m.desktop" "$D/share/applications/$m.desktop"
done
if want nemo-files; then
  install -Dm644 "$M/nemo-files/config/nemo.portal" "$D/share/xdg-desktop-portal/portals/nemo.portal"
  service "$M/nemo-files/config/org.freedesktop.impl.portal.desktop.nemo.service" nemo-files
  # no org.freedesktop.FileManager1 here: Nautilus owns that system file, and «show in folder» for every
  # app is each person's choice (`nemo-files install` asks and puts it into the home folder)
fi
if want nemo-greet; then
  # the login screen finds bin/nemo-greet from libexec/nemo and its fonts in share/nemo-greet/fonts (docs/PACKAGING.md)
  install -Dm755 "$HERE/bin/nemo-greet" "$D/bin/nemo-greet"
  install -Dm755 "$M/nemo-greet/greet-session" "$D/libexec/nemo/greet-session"
  install -Dm644 -t "$D/share/nemo-greet/fonts" "$M/nemo-greet/fonts/Tiny5.ttf" "$M/nemo-greet/fonts/DotGothic16-Regular.ttf"
  licence nemo-greet
fi

# Nemo was called yami: an installation under the old name moves to the new one without losing the settings.
# legacy_migrate  (first, quiet): the folders of settings, state, cache and data move; the old shell is not touched
# legacy_stop     (when the new shell starts): the old shell and agents stop
# legacy_cleanup  (after the new one is in place): the old commands, launchers and glue files go
# All three are safe to run again and do nothing on a machine that never had the old name.
# shellcheck shell=bash

legacy_present() { [[ -e $BIN/yami-shell || -d ${XDG_CONFIG_HOME:-$HOME/.config}/yami || -d ${XDG_STATE_HOME:-$HOME/.local/state}/yami ]]; }

# move the contents of $1 into $2 (what is already in $2 stays), then drop an emptied $1
legacy_merge() {
  local from=$1 to=$2 e skip=${LEGACY_EXCLUDE:-}
  local LEGACY_EXCLUDE=
  [[ -d $from ]] || return 0
  mkdir -p "$to"
  for e in "$from"/* "$from"/.[!.]*; do
    [[ -e $e || -L $e ]] || continue
    [[ -n $skip && ${e##*/} == "$skip" ]] && continue
    if [[ ! -e $to/${e##*/} && ! -L $to/${e##*/} ]]; then mv "$e" "$to/"
    elif [[ -d $e && -d $to/${e##*/} ]]; then legacy_merge "$e" "$to/${e##*/}"
    # a file the old shell wrote after the first move is newer than the one already carried over: it wins
    elif [[ -f $e && -f $to/${e##*/} && $e -nt $to/${e##*/} ]]; then mv -f "$e" "$to/"; fi
  done
  rmdir "$from" 2>/dev/null || true
}

legacy_migrate() {
  legacy_present || return 0
  local cfg="${XDG_CONFIG_HOME:-$HOME/.config}" state="${XDG_STATE_HOME:-$HOME/.local/state}" cache="${XDG_CACHE_HOME:-$HOME/.cache}" data="${XDG_DATA_HOME:-$HOME/.local/share}" m
  # the niri part stays until the new shell is in place: config.kdl includes it by the old path until the shell rewrites that line
  if [[ -x $BIN/nemo-shell ]]; then legacy_merge "$cfg/yami" "$cfg/nemo"; else LEGACY_EXCLUDE=niri legacy_merge "$cfg/yami" "$cfg/nemo"; fi
  legacy_merge "$state/yami" "$state/nemo"
  legacy_merge "$cache/yami" "$cache/nemo"
  legacy_merge "$data/yami" "$data/nemo"
  legacy_merge "$data/fonts/yami" "$data/fonts/nemo"
  # the module checkouts carry the module name
  for m in "$data"/nemo/modules/yami-*; do
    [[ -d $m ]] || continue
    [[ -e $data/nemo/modules/nemo-${m##*/yami-} ]] || mv "$m" "$data/nemo/modules/nemo-${m##*/yami-}"
  done
  [[ -d $data/nemo/src && -f $data/nemo/src/install.sh ]] || true
  # the niri parts the shell wrote: the file that config.kdl includes is rewritten by the shell at its start
  [[ -f $cfg/niri/.yami && ! -e $cfg/niri/.nemo ]] && mv "$cfg/niri/.yami" "$cfg/niri/.nemo"
  # settings written before 3.0
  [[ -f $cfg/yami-shell.json && ! -f $cfg/nemo/shell.json ]] && mv "$cfg/yami-shell.json" "$cfg/nemo/shell.json"
  return 0
}

legacy_stop() {
  [[ -e $BIN/yami-shell ]] || pgrep -x yami-shell >/dev/null || return 0
  pkill -f 'until yami-shell' 2>/dev/null
  pkill -x yami-shell 2>/dev/null
  pkill -x yami-agents 2>/dev/null
  pkill -f "yami-files --portal" 2>/dev/null
  sleep 0.3
  legacy_migrate
  return 0
}

legacy_cleanup() {
  local cfg="${XDG_CONFIG_HOME:-$HOME/.config}" data="${XDG_DATA_HOME:-$HOME/.local/share}" g f
  [[ -x $BIN/nemo-shell ]] || return 0
  rm -f "$BIN"/yami "$BIN"/yami-shell "$BIN"/yami-files "$BIN"/yami-view "$BIN"/yami-play "$BIN"/yami-shot "$BIN"/yami-agents "$BIN"/yami-setup
  rm -f "$data"/applications/yami-{files,view,play}.desktop "$data"/icons/hicolor/scalable/apps/yami-{files,view,play}.svg \
    "$data/xdg-desktop-portal/portals/yami.portal" "$data/xdg-desktop-portal/portals/yami-agents.portal" \
    "$data/dbus-1/services/org.freedesktop.impl.portal.desktop.yami.service" "$data/dbus-1/services/org.freedesktop.impl.portal.desktop.yamiagents.service" \
    "$cfg/environment.d/60-yami-path.conf" "$cfg/systemd/user/niri.service.d/60-yami-path.conf" "$cfg/fontconfig/conf.d/50-yami-pixel.conf" \
    "$cfg/autostart/yami-shell.desktop" "$cfg/qt6ct/qss/yami.qss" "$cfg/qt6ct/colors/yami.conf" "$cfg/btop/themes/yami.theme"
  find "$data/icons" -maxdepth 1 -type d -regextype posix-extended -regex '.*/yami-[0-9a-f]{6}(-l)?' -exec rm -rf {} + 2>/dev/null
  # the line that put our colours into kitty and GTK: the new shell writes its own
  [[ -f $cfg/kitty/kitty.conf ]] && sed -i '/^include yami.conf$/d; /^# YAMI/d' "$cfg/kitty/kitty.conf" && rm -f "$cfg/kitty/yami.conf"
  for g in "$cfg/gtk-3.0" "$cfg/gtk-4.0"; do
    rm -f "$g/yami.css"
    [[ -f $g/gtk.css ]] && sed -i '/^@import url("yami.css");$/d; /^\/\* yami: its colours live in yami.css/d' "$g/gtk.css"
  done
  # the file picker and the mime defaults pointed at the old launchers
  for f in "$cfg/mimeapps.list" "$data/applications/mimeapps.list"; do
    [[ -f $f ]] && sed -i 's/yami-\(files\|view\|play\)\.desktop/nemo-\1.desktop/g' "$f"
  done
  return 0
}

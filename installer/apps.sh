# Our modules, each one only when chosen (their files come from the module's own repository): nemo-files (folders, the file dialog of every app through the
# portal, «show in folder»), nemo-view (pictures), nemo-play (video and music). Their .desktop files
# and the defaults for their MIME types; unchecked over an earlier install, an app goes again with
# its defaults (mimeapps.list keeps every other entry), nemo-files with its portal and D-Bus files.
# The binaries themselves come from build.sh.
# shellcheck shell=bash

plan_apps() {
  if on shot; then :
  elif may_drop && app_here nemo-shot; then add "$(t "скриншоты: убрать nemo-shot (не выбран)" "screenshots: remove nemo-shot (not chosen)" "capturas: remover o nemo-shot (não escolhido)")" drop_app nemo-shot; fi
  # the services (nemo-agents) are part of the shell: their portal and D-Bus files go with it
  on core && add "$(t "службы шелла: пароли, показ экрана, флешки" "the shell's services: passwords, screen sharing, drives" "serviços do shell: senhas, compartilhar a tela, pendrives")" do_agents
  if on files; then add "$(t "файлы: nemo-files и диалог выбора файлов" "files: nemo-files and the file dialog" "arquivos: nemo-files e diálogo de arquivos")" do_files
  elif may_drop && app_here nemo-files; then add "$(t "файлы: убрать nemo-files (не выбран)" "files: remove nemo-files (not chosen)" "arquivos: remover o nemo-files (não escolhido)")" drop_files; fi
  if on view; then add "$(t "картинки: nemo-view" "images: nemo-view" "imagens: nemo-view")" do_view
  elif may_drop && app_here nemo-view; then add "$(t "картинки: убрать nemo-view (не выбран)" "images: remove nemo-view (not chosen)" "imagens: remover o nemo-view (não escolhido)")" drop_app nemo-view; fi
  if on play; then add "$(t "плеер: nemo-play" "player: nemo-play" "player: nemo-play")" do_play
  elif may_drop && app_here nemo-play; then add "$(t "плеер: убрать nemo-play (не выбран)" "player: remove nemo-play (not chosen)" "player: remover o nemo-play (não escolhido)")" drop_app nemo-play; fi
  return 0
}
# unticked modules go only when someone chose: a base install over an old one removes nothing
may_drop() { ((UPDATE || FULL || APPLY)); }
app_here() { [[ -e $BIN/$1 || -e ${XDG_DATA_HOME:-$HOME/.local/share}/applications/$1.desktop ]]; }
# the .desktop file and the default for its MimeType list
app_desktop() {
  local data="${XDG_DATA_HOME:-$HOME/.local/share}"
  # на чистой системе ~/.config ещё нет, а xdg-mime его не создаёт
  mkdir -p "$CFG" "$data/applications"
  local src
  src="$(m_src "$1")" || return 1
  install -Dm644 "$src/config/$1.desktop" "$data/applications/$1.desktop" || return 1
  command -v xdg-mime >/dev/null || return 0
  # shellcheck disable=SC2046 # one argument per MIME type
  xdg-mime default "$1.desktop" $(sed -n 's/^MimeType=//p' "$src/config/$1.desktop" | tr ';' ' ')
}
# pictures open in nemo-view, video and music in nemo-play
do_view() { app_desktop nemo-view; }
do_play() { app_desktop nemo-play; }
# an app that is no longer chosen: its binary, .desktop and the defaults pointing at it
drop_app() {
  local data="${XDG_DATA_HOME:-$HOME/.local/share}"
  rm -f -- "$BIN/$1" "$data/applications/$1.desktop" "$data/icons/hicolor/scalable/apps/$1.svg"
  mime_forget "$1.desktop"
}
# mimeapps.list without the given .desktop (xdg-mime default wrote it); other entries stay as they are
mime_forget() {
  local f="$CFG/mimeapps.list" tmp
  [[ -f $f ]] || return 0
  tmp="$f.nemo-new"
  awk -v app="$1" '
    /^\[/ || /^#/ || index($0, "=") == 0 { print; next }
    {
      eq = index($0, "=")
      n = split(substr($0, eq + 1), a, ";")
      out = ""; hit = 0
      for (i = 1; i <= n; i++) {
        if (a[i] == app) hit = 1
        else if (a[i] != "") out = out a[i] ";"
      }
      if (!hit) { print; next }
      if (out != "") print substr($0, 1, eq) out
    }
  ' "$f" >"$tmp" && { cmp -s "$tmp" "$f" && rm -f -- "$tmp" || mv -f -- "$tmp" "$f"; }
}

# niri-portals.conf follows what is installed: file dialogs from nemo-files, the screen-share picker from
# nemo-agents; a file of your own (no «# nemo:» line) stays yours. The portal frontend reads it only at
# its start, so a change restarts it
portal_sync() {
  local pc="$CFG/xdg-desktop-portal/niri-portals.conf" before=""
  [[ -f $pc ]] && before="$(cat "$pc")"
  if [[ ! -x $BIN/nemo-files && ! -x $BIN/nemo-agents ]]; then
    head -1 "$pc" 2>/dev/null | grep -q '^# nemo:' && rm -f -- "$pc"
  else
    "$BIN/nemo-shell" config portals
  fi
  [[ "$before" == "$(cat "$pc" 2>/dev/null)" ]] && return 0
  if ((SYSTEMD)) && systemctl --user is-active -q xdg-desktop-portal 2>/dev/null; then
    busctl --user call org.freedesktop.DBus /org/freedesktop/DBus org.freedesktop.DBus ReloadConfig >/dev/null 2>&1
    systemctl --user restart xdg-desktop-portal
  fi
  return 0
}

# ── services: the screen-share portal backend and the agent process ─────────
do_agents() {
  local data="${XDG_DATA_HOME:-$HOME/.local/share}" src="$ROOT/shell/agents"
  install -Dm644 "$src/config/nemo-agents.portal" "$data/xdg-desktop-portal/portals/nemo-agents.portal" || return 1
  mkdir -p "$data/dbus-1/services"
  sed "s|@BIN@|$BIN/nemo-agents|" "$src/config/org.freedesktop.impl.portal.desktop.nemoagents.service" \
    >"$data/dbus-1/services/org.freedesktop.impl.portal.desktop.nemoagents.service"
  # работающая служба переходит на новый бинарь, как только свободна, остановленная запускается
  ((RUNNING_NIRI)) && "$BIN/nemo-agents" --reload </dev/null >/dev/null 2>&1
  portal_sync
}
# ── files: nemo-files as the folder handler, the portal file dialog ─────────
do_files() {
  local data="${XDG_DATA_HOME:-$HOME/.local/share}" src
  src="$(m_src nemo-files)" || return 1
  app_desktop nemo-files || return 1
  # xdg-desktop-portal: FileChooser → nemo-files (browsers, Telegram, GTK/KDE apps)
  install -Dm644 "$src/config/nemo.portal" "$data/xdg-desktop-portal/portals/nemo.portal"
  mkdir -p "$data/dbus-1/services"
  sed "s|@BIN@|$BIN/nemo-files|" "$src/config/org.freedesktop.impl.portal.desktop.nemo.service" \
    >"$data/dbus-1/services/org.freedesktop.impl.portal.desktop.nemo.service"
  # «показать в папке» из браузеров и прочих: org.freedesktop.FileManager1
  sed "s|@BIN@|$BIN/nemo-files|" "$src/config/org.freedesktop.FileManager1.service" \
    >"$data/dbus-1/services/org.freedesktop.FileManager1.service"
  # Firefox and its forks pick the portal dialog only with this pref
  # profiles from profiles.ini too (they may live outside the browser folder)
  local prof
  while IFS= read -r prof; do
    [[ -f $prof/prefs.js ]] || continue
    grep -qs 'use-xdg-desktop-portal.file-picker' "$prof/user.js" ||
      printf '// nemo: file dialogs through the portal (nemo-files)\nuser_pref("widget.use-xdg-desktop-portal.file-picker", 1);\n' >>"$prof/user.js"
  done < <("$BIN/nemo-shell" recolor --profiles 2>/dev/null)
  # the backend left from the previous build moves to the new binary as soon as no screencast or
  # file dialog is open (a running stream is not cut), and is started when it is not running:
  # xdg-desktop-portal takes ScreenCast's source types from the live backend, without it OBS and
  # browsers see nothing to capture. Only in a niri session or over a running backend: installed
  # from another desktop, it would take «show in folder» (FileManager1) there until logout
  if ((RUNNING_NIRI)) || pgrep -f '^[^ ]*nemo-files --portal$' >/dev/null; then
    "$BIN/nemo-files" --portal-reload </dev/null >/dev/null 2>&1
  fi
  portal_sync
}
# nemo-files unchecked: the backend stops, the portal and D-Bus files go (dialogs fall back to GTK)
drop_files() {
  local data="${XDG_DATA_HOME:-$HOME/.local/share}"
  pkill -f "^$BIN/nemo-files --portal" 2>/dev/null
  drop_app nemo-files
  rm -f -- "$data/xdg-desktop-portal/portals/nemo.portal" "$data/dbus-1/services/org.freedesktop.impl.portal.desktop.nemo.service" \
    "$data/dbus-1/services/org.freedesktop.FileManager1.service"
  portal_sync
}

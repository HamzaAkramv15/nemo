# ./install.sh --uninstall. The shell, its apps and files go, backups come
# back, the login screen goes (login.sh), the settings file stays.
# shellcheck shell=bash

uninstall() {
  printf '\e[H\e[J'
  banner 1
  say "$(t "удаляю NEMO" "removing NEMO" "removendo NEMO")"
  ((DRY)) && { note "--dry-run"; return; }
  if ((!YES)); then
    printf '  %s%s%s %s[%s]%s ' "$T" "$(t "Удалить nemo с этой машины? Из конфига niri уйдёт только строка nemo, настройки останутся в ~/.config/nemo." "Remove nemo from this machine? Only nemo's line leaves the niri config, the settings stay in ~/.config/nemo." "Remover o nemo desta máquina? Só a linha do nemo sai da configuração do niri; as configurações ficam em ~/.config/nemo.")" "$R" "$F" "$(t "y — да, enter — нет" "y yes, enter no" "y sim, enter não")" "$R"
    local a
    read -r a || a=n
    [[ $a =~ ^[yYдД] ]] || bye
  fi
  # первым: скрипт экрана входа лежит в том, что удаляется ниже
  uninstall_login
  # строки Nemo из конфига niri (всё остальное в нём — твоё и остаётся), пока бинарь ещё есть
  [[ -x $BIN/nemo-shell ]] && "$BIN/nemo-shell" niri disconnect
  pgrep -x nemo-shell >/dev/null && "$BIN/nemo-shell" msg quit 2>/dev/null
  [[ -x $HOME/.local/lib/nemo-niri/nemo-niri.sh ]] && "$HOME/.local/lib/nemo-niri/nemo-niri.sh" off
  rm -rf "$HOME/.local/lib/nemo-niri" "$HOME/.local/lib/nemo"
  rm -f "$BIN/nemo-shell" "$BIN/nemo" "$STATE/install.json" "$STATE/no-monocraft" "$STATE/update.json" "$STATE/update-keys" "$CFG/environment.d/60-nemo-path.conf" "$CFG/systemd/user/niri.service.d/60-nemo-path.conf" "$CFG/fontconfig/conf.d/50-nemo-pixel.conf" \
    "$CFG"/niri/nemo-{local,binds,fx,colors,cursor}.kdl "$CFG/niri/.nemo"
  local data="${XDG_DATA_HOME:-$HOME/.local/share}"
  pkill -f "$BIN/nemo-files --portal" 2>/dev/null
  pkill -x nemo-agents 2>/dev/null
  rm -f "$BIN/nemo-fedora-update" "$BIN/nemo-setup" "$BIN/nemo-shot" "$BIN/nemo-agents" "$data/xdg-desktop-portal/portals/nemo-agents.portal" "$data/dbus-1/services/org.freedesktop.impl.portal.desktop.nemoagents.service" "$STATE/first-run" "$STATE/source"
  # ~/.config/nemo: its niri part goes, the settings (*.json), themes, templates and scripts stay
  rm -rf "$HOME/.local/share/nemo/modules" "$CFG/nemo/niri"
  rm -f "$BIN/nemo-files" "$BIN/nemo-view" "$BIN/nemo-play" "$data/applications/nemo-files.desktop" "$data/applications/nemo-view.desktop" "$data/applications/nemo-play.desktop" "$data/xdg-desktop-portal/portals/nemo.portal" \
    "$data/dbus-1/services/org.freedesktop.impl.portal.desktop.nemo.service" "$data/dbus-1/services/org.freedesktop.FileManager1.service" \
    "$CFG/xdg-desktop-portal/niri-portals.conf" "$CFG/btop/themes/nemo.theme"
  # the apps' own icons recolor drew in the palette
  rm -f "$data/icons/hicolor/scalable/apps/nemo-files.svg" "$data/icons/hicolor/scalable/apps/nemo-view.svg" "$data/icons/hicolor/scalable/apps/nemo-play.svg"
  # pixel cursors: nemo-<accent>, and nemo-<accent>-l for the light theme
  find "$data/icons" -maxdepth 1 -type d -regextype posix-extended -regex '.*/nemo-[0-9a-f]{6}(-l)?' -exec rm -rf {} + 2>/dev/null
  # the MangoHud config recolor wrote (one without its mark is the user's own)
  head -1 "$CFG/MangoHud/MangoHud.conf" 2>/dev/null | grep -qE '^# nemo: written by (tools/theme.py|nemo recolor)' && rm -f "$CFG/MangoHud/MangoHud.conf"
  rm -rf "$HOME/.local/share/fonts/nemo" "$HOME/.local/share/nemo" "$CFG/qt6ct/qss/nemo"
  # the one file 2.7 could put outside the home folder for NVIDIA
  { [[ -n $SU ]] || ((EUID == 0)); } && $SU rm -f /etc/nvidia/nvidia-application-profiles-rc.d/50-niri-vram.json
  rm -f "$STATE/packages-needed" "$STATE/components-known" "$STATE/settings-newer.json" "$STATE/theme.py" "$STATE/restyle.json" "$STATE/templates.json"
  rm -rf -- "$STATE/built" "$STATE/snapshots"
  mime_forget nemo-files.desktop
  mime_forget nemo-view.desktop
  mime_forget nemo-play.desktop
  # the build cache (gigabytes); nemo-niri sources only without patch work in them
  rm -rf -- "$CACHE/target" "$CACHE/sddm-nemo" "$CACHE/nemo-cursor" "$CACHE/update.log" "$CACHE/update.lock"
  local src="${NEMO_NIRI_SRC:-$CACHE/niri-src}"
  if niri_src_plain "$src"; then
    rm -rf -- "$src"
  elif [[ -d $src ]]; then
    note "$(t "в {} твоя работа над патчами — оставил" "your patch work is in {}: kept" "seus patches estão em {}: preservados" "$src")"
  fi
  rm -f "$CFG/qt6ct/qss/nemo.qss" "$CFG/qt6ct/colors/nemo.conf"
  # GTK: our nemo.css and its import line; a gtk.css nemo wrote whole before 3.0 goes too
  local g
  for g in "$CFG/gtk-3.0" "$CFG/gtk-4.0"; do
    rm -f "$g/nemo.css"
    [[ -f $g/gtk.css ]] || continue
    if head -1 "$g/gtk.css" | grep -q '^/\* NEMO'; then rm -f "$g/gtk.css"
    else sed -i '/^@import url("nemo.css");$/d; /^\/\* nemo: its colours live in nemo.css/d' "$g/gtk.css"; fi
  done
  fc-cache -f >/dev/null 2>&1
  if [[ -f "$CFG/kitty/kitty.conf" ]]; then
    sed -i '/^# NEMO/d; /^include nemo.conf$/d' "$CFG/kitty/kitty.conf"
    rm -f "$CFG/kitty/nemo.conf"
  fi
  # nemo before 3.0 replaced the whole niri folder and kept the old one next to it
  local b
  b="$(ls -d "$CFG"/niri.bak-* 2>/dev/null | sort | tail -n1)"
  # (only while config.kdl is still nemo's, the old one or its starting one: not a config you made since)
  if [[ -n $b ]] && grep -qE '"nemo-binds.kdl"|^// niri.s config: yours. Nemo adds itself' "$CFG/niri/config.kdl" 2>/dev/null; then
    mv "$CFG/niri" "$CFG/niri.nemo-$STAMP" && mv "$b" "$CFG/niri"
    say "$(t "niri: вернул" "niri: restored" "niri: restaurado") $b"
  fi
  # the copies of the other programs' files that we kept (before the rename they ended in .bak-yami)
  while IFS= read -r b; do mv -f "$b" "${b%.bak-nemo}"; done < <(find "$CFG" -maxdepth 3 -name '*.bak-nemo' 2>/dev/null)
  while IFS= read -r b; do mv -f "$b" "${b%.bak-yami}"; done < <(find "$CFG" -maxdepth 3 -name '*.bak-yami' 2>/dev/null)
  say "$(t "готово. настройки шелла остались в" "done. shell settings are kept in" "concluído. as configurações do shell ficam em") $CFG/nemo"
  exit 0
}

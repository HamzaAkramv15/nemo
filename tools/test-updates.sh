#!/usr/bin/env bash
# Install and update scenarios, end to end and with no network: a fake GitHub of bare repositories with
# signed tags, a throwaway HOME for every scenario, real builds and the real updater (`nemo update
# apply`). Nothing reaches the running session: env -i, its own XDG_RUNTIME_DIR, no D-Bus and no
# NIRI_SOCKET; sudo, systemctl, pkill and the like are stubs that write down what was asked of them
# (root commands are never run, so the root half of the login screen is checked by what it was asked).
# The sources are this working tree and the modules next to it, edits included.
#   tools/test-updates.sh [--list] [scenario …]     all of them by default; logs in $NEMO_TEST_DIR/logs
# The scenario «release» runs tools/release.sh itself (make, --undo, make, --push) with a stand-in gh.
# nemo-niri is a stub (a real build is 5–25 minutes and 4 GB); needs a packaged niri for `niri validate`.
set -uo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODS="${NEMO_MODULES_DIR:-$ROOT/../nemo-modules}"
WORK="${NEMO_TEST_DIR:-${XDG_CACHE_HOME:-$HOME/.cache}/nemo-test}"
GH="$WORK/github/HamzaAkramv15"
SRC="$WORK/work"
SHIMS="$WORK/shims"
LOGS="$WORK/logs"
H="$WORK/home"
REAL_HOME="$HOME"
# the modules, each its own repository; the shell's services (nemo-agents) come from the shell's tree
MODULES=(nemo-files nemo-view nemo-play nemo-shot nemo-greet nemo-niri)
BINS=(nemo-shell nemo-files nemo-view nemo-play nemo-shot nemo-agents)
PASS=0 FAIL=0 FAILED_NAMES=()

say() { printf '\e[1m── %s\e[0m\n' "$*"; }
ok() { PASS=$((PASS + 1)); printf '  \e[32m✓\e[0m %s\n' "$1"; }
no() { FAIL=$((FAIL + 1)); FAILED_NAMES+=("$SCEN: $1"); printf '  \e[31m✗ %s\e[0m\n' "$1"; [[ -n ${2:-} ]] && printf '    %s\n' "$2"; }
# check "what" command…
check() { local d=$1; shift; if "$@" >/dev/null 2>&1; then ok "$d"; else no "$d" "$*"; fi; }
nocheck() { local d=$1; shift; if "$@" >/dev/null 2>&1; then no "$d" "$*"; else ok "$d"; fi; }
exists() { [[ -e $1 ]]; }
missing() { [[ ! -e $1 ]]; }
contains() { grep -qF -- "$2" "$1"; }

# ── the fake world ──────────────────────────────────────────────────────────
make_shims() {
  mkdir -p "$SHIMS"
  local c
  for c in systemctl busctl pkill gsettings notify-send update-desktop-database gtk-update-icon-cache; do
    printf '#!/bin/sh\necho "%s $*" >>"$HOME/calls.log"\nexit 0\n' "$c" >"$SHIMS/$c"
  done
  # not active, not running: nothing to restart
  printf '#!/bin/sh\necho "systemctl $*" >>"$HOME/calls.log"\ncase "$*" in *is-active*|*is-enabled*) exit 1;; esac\nexit 0\n' >"$SHIMS/systemctl"
  printf '#!/bin/sh\necho "pgrep $*" >>"$HOME/calls.log"\nexit 1\n' >"$SHIMS/pgrep"
  printf '#!/bin/sh\nexit 0\n' >"$SHIMS/greetd"
  # no network in the tests: the counter is recorded, anything else fails as if offline
  cat >"$SHIMS/curl" <<'SH'
#!/bin/sh
echo "curl $*" >>"$HOME/calls.log"
# the installer's own «is the network up» check; everything else is offline
case "$*" in *nemo-installs.txt* | *" https://github.com") exit 0 ;; esac
exit 22
SH
  # root is never used: what would run as root is written down. The installer's root shell (a loop
  # reading commands) is answered «done» for each command
  cat >"$SHIMS/sudo" <<'SH'
#!/bin/bash
echo "sudo $*" >>"$HOME/root.log"
args=("$@")
[[ ${args[0]:-} == -A ]] && { args=("${args[@]:1}"); echo "askpass" >>"$HOME/root.log"; }
[[ ${args[0]:-} == -k || ${args[0]:-} == -n ]] && exit 0
if [[ ${args[0]:-} == bash && ${args[1]:-} == -c ]]; then
  while IFS= read -r p && IFS= read -r c; do
    echo "root: $c" >>"$HOME/root.log"
    # a package index older than the mirrors (an Arch not upgraded for a while): what pacman says then
    if [[ -e $HOME/stale-index && $c == *"pacman -S"* ]]; then
      echo "error: failed retrieving file 'niri-26.04-1-x86_64.pkg.tar.zst' from mirror.example : The requested URL returned error: 404" >>"/proc/$p/fd/2"
      echo 1
    else
      echo 0
    fi
  done
  exit 0
fi
exit 0
SH
  chmod +x "$SHIMS"/*
}

# every file of a working tree that git would see, edits included
snap_tree() { # from to
  mkdir -p "$2"
  (
    cd "$1" || exit 1
    git ls-files -z --cached --others --exclude-standard | while IFS= read -r -d '' f; do
      [[ -e $f || -L $f ]] || continue
      mkdir -p "$2/$(dirname "$f")" && cp -a -- "$f" "$2/$f"
    done
  )
}
g() { GIT_AUTHOR_NAME=t GIT_AUTHOR_EMAIL=t@t GIT_COMMITTER_NAME=t GIT_COMMITTER_EMAIL=t@t git -c gpg.format=ssh -c "user.signingkey=$WORK/key.pub" -c commit.gpgsign=false "$@"; }
commit_tag() { # dir tag [key]
  g -C "$1" add -A && g -C "$1" commit -q -m "$2" --allow-empty && g -C "$1" tag -s -m "$2" "$2" ${3:+-c "user.signingkey=$3"}
}
push() { g -C "$SRC/$1" push -q --force origin main --tags; }
# a version in Cargo.toml and in the crate's own line of Cargo.lock
set_crate_ver() { # dir name version
  [[ -f $1/Cargo.toml ]] || return 0
  sed -i '0,/^version = "/s/^version = ".*/version = "'"$3"'"/' "$1/Cargo.toml"
  [[ -f $1/Cargo.lock ]] &&   sed -i '/^name = "'"$2"'"$/{n;s/^version = .*/version = "'"$3"'"/}' "$1/Cargo.lock"
}
gh_name() { case $1 in nemo) echo nemo ;; nemo-*) echo "nemo-${1#nemo-}" ;; *) echo "$1" ;; esac; } # the name on the fake server, as on GitHub
new_repo() { # name from-tree
  rm -rf "${SRC:?}/$1"
  git init -q -b main "$SRC/$1"
  snap_tree "$2" "$SRC/$1"
  git init -q --bare -b main "$GH/$(gh_name "$1").git"
  git -C "$GH/$(gh_name "$1").git" config uploadpack.allowFilter true
  git -C "$SRC/$1" remote add origin "$GH/$(gh_name "$1").git"
}
# the shell and its services at a version (both crates, one Cargo.lock)
shell_ver() { # tree version
  set_crate_ver "$1/shell" nemo-shell "$2"
  [[ -f $1/shell/agents/Cargo.toml ]] || return 0
  sed -i '0,/^version = "/s/^version = ".*/version = "'"$2"'"/' "$1/shell/agents/Cargo.toml"
  sed -i '/^name = "nemo-agents"$/{n;s/^version = .*/version = "'"$2"'"/}' "$1/shell/Cargo.lock"
}
# the shell at a version, with the modules.lock of that version
shell_tag() { # version
  local d="$SRC/nemo" r
  printf '%s\n' "$1" >"$d/VERSION"
  shell_ver "$d" "$1"
  commit_tag "$d" "v$1"
  push nemo
}
module_tag() { # module version
  set_crate_ver "$SRC/$1" "$1" "$2"
  commit_tag "$SRC/$1" "v$2"
  push "$1"
}
lock_all() { # version
  : >"$SRC/nemo/modules.lock"
  local m
  for m in "${MODULES[@]}"; do printf '%s v%s\n' "$m" "$1" >>"$SRC/nemo/modules.lock"; done
}
world() {
  say "building the fake GitHub"
  rm -rf "$WORK/github" "$SRC"
  mkdir -p "$GH" "$SRC" "$LOGS"
  [[ -f $WORK/key ]] || ssh-keygen -q -t ed25519 -N "" -f "$WORK/key" -C test
  [[ -f $WORK/rogue ]] || ssh-keygen -q -t ed25519 -N "" -f "$WORK/rogue" -C rogue
  make_shims
  new_repo nemo "$ROOT"
  px_tags
  printf 'test namespaces="git,nemo-release" %s\n' "$(cut -d' ' -f1,2 "$WORK/key.pub")" >"$SRC/nemo/keys/allowed_signers"
  rm -f "$SRC/nemo/CLAUDE.md" "$SRC/nemo/AGENTS.md" "$SRC/nemo/GEMINI.md"
  local m
  for m in "${MODULES[@]}"; do
    new_repo "$m" "$MODS/$m"
    [[ $m == nemo-niri ]] && stub_niri
    module_tag "$m" 3.0.0
  done
  lock_all 3.0.0
  shell_tag 3.0.0
  ok "repositories at 3.0.0"
}
# nemo-px comes through cargo from the shell's repository with its real history: the modules' Cargo.lock
# names a commit of it (the tag px-vX.Y.Z), which the test history of the shell does not have
px_tags() {
  local px
  for px in $(git -C "$ROOT" tag -l 'px-v*'); do
    git -C "$GH/nemo.git" fetch -q --no-tags "$ROOT" "refs/tags/$px:refs/tags/$px"
  done
}
# nemo-niri without its 5–25 minute build: the same commands, a stand-in binary
stub_niri() {
  rm -rf "${SRC:?}/nemo-niri/patches"
  cat >"$SRC/nemo-niri/nemo-niri.sh" <<'SH'
#!/usr/bin/env bash
VERSION="26.04-nemo.test"
case ${1:-} in
  build)
    mkdir -p "$HOME/.local/lib/nemo-niri/current"
    printf '#!/bin/sh\necho "niri 26.04-nemo.test"\n' >"$HOME/.local/lib/nemo-niri/current/niri"
    chmod +x "$HOME/.local/lib/nemo-niri/current/niri"
    cp "$0" "$HOME/.local/lib/nemo-niri/nemo-niri.sh" ;;
  off | on | status) ;;
esac
SH
  chmod +x "$SRC/nemo-niri/nemo-niri.sh"
}

# ── a throwaway machine ─────────────────────────────────────────────────────
machine() { # fresh HOME; the build cache is shared between scenarios
  rm -rf "${H:?}"
  mkdir -p "$H/run" "$H/.cache/nemo" "$WORK/target"
  chmod 700 "$H/run"
  ln -s "$WORK/target" "$H/.cache/nemo/target"
}
sandbox() { # command…: everything the installer and the updater run goes through here
  env -i -C "$H" HOME="$H" USER=tester LOGNAME=tester LANG=en_US.UTF-8 TERM=dumb SHELL=/bin/bash \
    PATH="$SHIMS:$H/.local/bin:$REAL_HOME/.cargo/bin:/usr/bin:/bin" \
    XDG_RUNTIME_DIR="$H/run" \
    CARGO_HOME="$REAL_HOME/.cargo" RUSTUP_HOME="$REAL_HOME/.rustup" CARGO_NET_OFFLINE=true CARGO_NET_GIT_FETCH_WITH_CLI=true \
    CARGO_PROFILE_RELEASE_LTO=off CARGO_PROFILE_RELEASE_CODEGEN_UNITS=256 CARGO_PROFILE_RELEASE_OPT_LEVEL=0 CARGO_PROFILE_RELEASE_STRIP=false CARGO_BUILD_JOBS=8 \
    GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0="url.file://$GH/.insteadOf" GIT_CONFIG_VALUE_0="https://github.com/HamzaAkramv15/" \
    "${SANDBOX_ENV[@]}" "$@"
}
SANDBOX_ENV=()
log_for() { printf '%s/%s.log' "$LOGS" "$1"; }
# bootstrap as the README says: the script read from a pipe, not from the repository
first_install() { # extra flags…
  SANDBOX_ENV+=()
  sandbox bash -c 'cat "$1" | bash -s -- "${@:2}"' _ "$SRC/nemo/install.sh" --lang en "$@" >>"$(log_for "$SCEN")" 2>&1
}
inst() { sandbox bash "$H/.local/share/nemo/src/install.sh" --lang en "$@" >>"$(log_for "$SCEN")" 2>&1; }
nemo() { sandbox "$H/.local/bin/nemo-shell" "$@" >>"$(log_for "$SCEN")" 2>&1; }
apply() { inst --apply "$1" --restyle "${2:-none}"; }
state() { printf '%s/.local/state/nemo/%s' "$H" "$1"; }
ver_of() { sandbox "$H/.local/bin/$1" --version 2>/dev/null | awk '{print $2}'; }
is_ver() { [[ "$(ver_of "$1")" == "$2" ]]; }
installed_version() { sed -n 's/.*"version": *"\([^"]*\)".*/\1/p' "$(state install.json)"; }
components() { sed -n 's/.*"components": *"\([^"]*\)".*/\1/p' "$(state install.json)"; }
has_comp() { [[ " $(components) " == *" $1 "* ]]; }
count_hits() { grep -c 'curl .*nemo-installs.txt' "$H/calls.log" 2>/dev/null || true; }
data() { printf '%s/.local/share/%s' "$H" "$1"; }
cfg() { printf '%s/.config/%s' "$H" "$1"; }
bin_sums() { (cd "$H/.local/bin" && sha256sum -- "${BINS[@]}" 2>/dev/null); }
niri_ok() { sandbox /usr/bin/niri validate -c "$(cfg niri/config.kdl)"; }
# settings: ~/.config/nemo/shell.json, before 3.0 ~/.config/nemo-shell.json; one's own niri lines: in
# config.kdl since 3.0 (nemo only adds its include), in user.kdl before (nemo wrote config.kdl whole)
settings_file() { if [[ -f $(cfg nemo/shell.json) ]]; then cfg nemo/shell.json; else cfg nemo-shell.json; fi; }
user_mark() {
  sandbox "$H/.local/bin/nemo-shell" config set barMargin 13
  if grep -q 'nemo/niri/nemo.kdl' "$(cfg niri/config.kdl)" 2>/dev/null; then printf '// my tweak\n' >>"$(cfg niri/config.kdl)"; else printf '// my tweak\n' >>"$(cfg niri/user.kdl)"; fi
}
user_kept() {
  # -q: a match in either file is enough, the other may be missing
  grep -q '"barMargin": *13' "$(settings_file)" && grep -qsF '// my tweak' "$(cfg niri/config.kdl)" "$(cfg niri/user.kdl)"
}
publish() { # version: every module and the shell move to it
  local m
  for m in "${MODULES[@]}"; do module_tag "$m" "$1"; done
  lock_all "$1"
  shell_tag "$1"
}
update() { sandbox "$H/.local/bin/nemo-shell" update apply >>"$(log_for "$SCEN")" 2>&1; }
reset_world() { world >/dev/null; }

# ── scenarios ───────────────────────────────────────────────────────────────
t_base() {
  machine
  check "the first install finishes" first_install --yes
  check "the shell is in place" is_ver nemo-shell 3.0.0
  # the first-run setup is part of the shell now (welcome.rs)
  nocheck "no separate setup window" exists "$H/.local/bin/nemo-setup"
  check "nemo command" test -L "$H/.local/bin/nemo"
  check "no module was installed" missing "$H/.local/bin/nemo-files"
  check "the record says core" has_comp core
  check "the shell's services come with the base" is_ver nemo-agents 3.0.0
  check "  their screen-share backend" exists "$(data xdg-desktop-portal/portals/nemo-agents.portal)"
  check "the first-run mark is set" exists "$(state first-run)"
  check "niri accepts the config" niri_ok
  check "the source is remembered" contains "$(state source)" "/.local/share/nemo/src"
  check "a rollback point is not made on a first install" missing "$(state snapshots)"
  [[ "$(count_hits)" == 1 ]] && ok "the counter asked once" || no "the counter asked once" "$(count_hits) requests"
  check "the counter request names only the URL" grep -qxE 'curl -fsSL -A nemo --connect-timeout 4 --max-time 8 -o /dev/null https://github.com/HamzaAkramv15/nemo/releases/download/installs/nemo-installs.txt' "$H/calls.log"
  check "no Python was started" sh -c "! grep -q python '$(log_for "$SCEN")'"
  # installing again: the same machine, repaired, nothing asked to count twice
  check "running the installer again is a repair" inst --yes
  check "the repair keeps the first-run mark as it was" exists "$(state first-run)"
  [[ "$(count_hits)" == 1 ]] && ok "the counter stays at one" || no "the counter stays at one" "$(count_hits) requests"
}
t_pieces() { # one module added at a time, one taken away
  machine
  first_install --yes
  : >"$(state first-run)"
  check "files alone" apply "files"
  check "  nemo-files is there" exists "$H/.local/bin/nemo-files"
  check "  its .desktop" exists "$(data applications/nemo-files.desktop)"
  check "  the file dialog portal" exists "$(data xdg-desktop-portal/portals/nemo.portal)"
  check "  file dialogs go to nemo-files" contains "$(cfg xdg-desktop-portal/niri-portals.conf)" "FileChooser=nemo;gtk;"
  nocheck "  no player" exists "$H/.local/bin/nemo-play"
  check "files and view" apply "files view"
  check "  nemo-view is there" exists "$H/.local/bin/nemo-view"
  check "view alone: files goes" apply "view"
  nocheck "  nemo-files is gone" exists "$H/.local/bin/nemo-files"
  nocheck "  the portal is gone" exists "$(data xdg-desktop-portal/portals/nemo.portal)"
  nocheck "  folders no longer open in it" contains "$(cfg mimeapps.list)" "nemo-files.desktop"
  check "  file dialogs fall back to GTK" contains "$(cfg xdg-desktop-portal/niri-portals.conf)" "FileChooser=gtk;"
  check "  the screen-share backend stays the shell's" contains "$(cfg xdg-desktop-portal/niri-portals.conf)" "ScreenCast=nemo-agents;gnome;"
  check "an old list that still names agents" apply "view agents"
  check "  the services stay" exists "$H/.local/bin/nemo-agents"
  check "everything" apply "files view play shot"
  for b in nemo-files nemo-view nemo-play nemo-shot nemo-agents; do check "  $b" exists "$H/.local/bin/$b"; done
  check "one package transaction per run" sh -c "grep -c 'root: .*pacman -S' '$H/root.log' | grep -qx 6"
  check "OCR comes with screenshots" sh -c "grep 'root: .*pacman -S' '$H/root.log' | tail -1 | grep -q tesseract"
  check "drives come with files" sh -c "grep 'root: .*pacman -S' '$H/root.log' | tail -1 | grep -q udisks2"
  check "the root shell got its password from askpass-less sudo here" contains "$H/root.log" "bash -c"
}
t_module() { # one module on its own through a whole update, then back, then forward again
  local m=$1 repo=nemo-$1 version_bin
  case $m in login) repo=nemo-greet ;; fork) repo=nemo-niri ;; esac
  version_bin=$repo
  reset_world
  machine
  first_install --yes
  check "installed $m on its own" apply "$m"
  user_mark
  [[ $m == files || $m == view || $m == play || $m == shot ]] && check "  $version_bin 3.0.0" is_ver "$version_bin" 3.0.0
  publish 3.1.0
  check "the update to 3.1.0 goes through" update
  check "  the record says 3.1.0" test "$(installed_version)" = 3.1.0
  check "  the shell is 3.1.0" is_ver nemo-shell 3.1.0
  check "  the choice is kept" has_comp "$m"
  [[ $m == files || $m == view || $m == play || $m == shot ]] && check "  $version_bin is 3.1.0" is_ver "$version_bin" 3.1.0
  check "  the shell's services moved with it" is_ver nemo-agents 3.1.0
  for b in files view play shot; do [[ $b == "$m" ]] || nocheck "  nemo-$b did not appear" exists "$H/.local/bin/nemo-$b"; done
  check "  settings and user.kdl kept" user_kept
  check "  niri accepts the config" niri_ok
  check "  a rollback point exists" sh -c "ls -d '$(state snapshots)'/*/ | grep -q ."
  [[ $m == fork ]] && check "  nemo-niri was built" exists "$H/.local/lib/nemo-niri/current/niri"
  [[ $m == login ]] && check "  the greeter is built, offered, not installed without root" sh -c "grep -q '^login ' '$(state packages-needed)' || true"
  check "the rollback goes through" nemo update rollback
  check "  the shell is 3.0.0 again" is_ver nemo-shell 3.0.0
  check "  the record says 3.0.0" test "$(installed_version)" = 3.0.0
  [[ $m == files || $m == view || $m == play || $m == shot ]] && check "  $version_bin is 3.0.0 again" is_ver "$version_bin" 3.0.0
  check "  settings kept through the rollback" user_kept
  check "updating again after a rollback" update
  check "  the shell is 3.1.0 again" is_ver nemo-shell 3.1.0
  [[ $m == files || $m == view || $m == play || $m == shot ]] && check "  $version_bin is 3.1.0 again" is_ver "$version_bin" 3.1.0
}
t_full() {
  reset_world
  machine
  check "the full install" first_install --yes --full
  check "  no first-run window" missing "$(state first-run)"
  for b in "${BINS[@]}"; do check "  $b" exists "$H/.local/bin/$b"; done
  check "  the terminal and the night light come with the base" sh -c "grep 'root: .*pacman -S' '$H/root.log' | grep -q 'kitty.*wlsunset'"
  check "  no Python among the packages" sh -c "! grep 'root: .*pacman -S' '$H/root.log' | grep -q python"
  user_mark
  publish 3.1.0
  check "the full update" update
  for b in nemo-files nemo-view nemo-play nemo-shot nemo-agents; do check "  $b 3.1.0" is_ver "$b" 3.1.0; done
  check "  settings kept" user_kept
  # the next release changes only one module: the others are not built again
  local before after
  before="$(stat -c %Y "$H/.local/bin/nemo-files")"
  module_tag nemo-view 3.2.0
  sed -i 's/^nemo-view .*/nemo-view v3.2.0/' "$SRC/nemo/modules.lock"
  shell_tag 3.2.0
  sleep 1
  check "a release that touches one module" update
  check "  nemo-view moved" is_ver nemo-view 3.2.0
  after="$(stat -c %Y "$H/.local/bin/nemo-files")"
  [[ $before == "$after" ]] && ok "  nemo-files was not rebuilt" || no "  nemo-files was not rebuilt"
  check "  nemo-files is still 3.1.0" is_ver nemo-files 3.1.0
  # uninstall takes it all back
  check "uninstall" inst --uninstall --yes
  for b in "${BINS[@]}"; do nocheck "  $b is gone" exists "$H/.local/bin/$b"; done
  nocheck "  the record is gone" exists "$(state install.json)"
  nocheck "  mimeapps forgot the apps" contains "$(cfg mimeapps.list)" "nemo-view.desktop"
  check "  settings stay" exists "$(settings_file)"
  check "  the counter mark stays" exists "$(state counted)"
}
t_failures() {
  local before
  reset_world
  # the package index is older than the mirrors: the first install stops at the packages, says what to
  # do and changes nothing (it used to try every package alone for many minutes, then call the base ready)
  machine
  : >"$H/stale-index"
  nocheck "a stale package index stops the first install" first_install --yes
  rm -f "$H/stale-index"
  check "  it says to update the system first" contains "$(log_for "$SCEN")" "sudo pacman -Syu"
  check "  one package transaction, not one per package" sh -c "grep -c 'root: .*pacman -S' '$H/root.log' | grep -qx 1"
  nocheck "  nothing was installed" exists "$H/.local/bin/nemo-shell"
  machine
  first_install --yes --full
  publish 3.1.0
  check "updated to 3.1.0" update

  # one module does not compile: nothing changes at all
  before="$(bin_sums)"
  printf '\ncompile_error!("broken on purpose");\n' >>"$SRC/nemo-play/src/main.rs"
  module_tag nemo-play 3.2.0
  sed -i 's/^nemo-play .*/nemo-play v3.2.0/' "$SRC/nemo/modules.lock"
  shell_tag 3.2.0
  nocheck "a module that does not compile fails the update" update
  [[ "$before" == "$(bin_sums)" ]] && ok "  every binary is the old one" || no "  every binary is the old one"
  check "  the record stays at 3.1.0" test "$(installed_version)" = 3.1.0

  # the next release fixes it: the update goes through
  git -C "$SRC/nemo-play" checkout -q HEAD~1 -- src/main.rs
  module_tag nemo-play 3.3.0
  sed -i 's/^nemo-play .*/nemo-play v3.3.0/' "$SRC/nemo/modules.lock"
  shell_tag 3.3.0
  check "a fixed release goes through" update
  check "  nemo-play 3.3.0" is_ver nemo-play 3.3.0

  # a tag signed by someone else
  before="$(bin_sums)"
  set_crate_ver "$SRC/nemo-view" nemo-view 3.4.0
  g -C "$SRC/nemo-view" add -A && g -C "$SRC/nemo-view" commit -q -m v3.4.0
  g -c "user.signingkey=$WORK/rogue.pub" -C "$SRC/nemo-view" tag -s -m rogue v3.4.0
  push nemo-view
  sed -i 's/^nemo-view .*/nemo-view v3.4.0/' "$SRC/nemo/modules.lock"
  shell_tag 3.4.0
  nocheck "a tag signed by a stranger stops the update" update
  [[ "$before" == "$(bin_sums)" ]] && ok "  every binary is the old one" || no "  every binary is the old one"
  check "  it says why" contains "$(log_for "$SCEN")" "signature"
  g -C "$SRC/nemo-view" tag -d v3.4.0 >/dev/null
  git -C "$GH/nemo-view.git" tag -d v3.4.0 >/dev/null

  # the module's repository is unreachable
  sed -i 's/^nemo-view .*/nemo-view v3.1.0/' "$SRC/nemo/modules.lock"
  shell_tag 3.5.0
  mv "$GH/nemo-view.git" "$GH/gone.git"
  rm -rf "$H/.local/share/nemo/modules/nemo-view"
  before="$(bin_sums)"
  nocheck "a missing repository stops the update" update
  [[ "$before" == "$(bin_sums)" ]] && ok "  every binary is the old one" || no "  every binary is the old one"
  mv "$GH/gone.git" "$GH/nemo-view.git"

  # nemo's new niri part is broken: the installer has already swapped the binaries and must put them back
  before="$(bin_sums)"
  local old_kdl
  old_kdl="$(cat "$(cfg niri/config.kdl)")"
  printf 'this is not kdl {{{\n' >>"$SRC/nemo/config/niri/base.kdl"
  shell_tag 3.6.0
  nocheck "a config niri rejects fails the update" update
  [[ "$before" == "$(bin_sums)" ]] && ok "  the snapshot put every binary back" || no "  the snapshot put every binary back"
  [[ "$old_kdl" == "$(cat "$(cfg niri/config.kdl)")" ]] && ok "  the config is the old one" || no "  the config is the old one"
  check "  niri accepts it" niri_ok
  git -C "$SRC/nemo" checkout -q HEAD~1 -- config/niri/base.kdl
  shell_tag 3.7.0
  check "a fixed release goes through" update
  check "  the shell is 3.7.0" is_ver nemo-shell 3.7.0
}
t_repair() { # the one-liner run again over a machine with chosen modules
  reset_world
  machine
  first_install --yes
  apply "files shot"
  rm -f "$(state first-run)"
  user_mark
  check "the installer run again" first_install --yes
  check "  modules stay" exists "$H/.local/bin/nemo-files"
  check "  the record keeps them" has_comp files
  check "  and nemo-shot" has_comp shot
  nocheck "  no first-run window appears" exists "$(state first-run)"
  check "  settings kept" user_kept
  publish 3.1.0
  check "the next update" update
  check "  still there" is_ver nemo-files 3.1.0
  check "  and nemo-shot" is_ver nemo-shot 3.1.0
  nocheck "  nothing else came" exists "$H/.local/bin/nemo-view"
}
t_privacy() {
  reset_world
  machine
  SANDBOX_ENV=(DO_NOT_TRACK=1)
  first_install --yes
  SANDBOX_ENV=()
  [[ "$(count_hits)" == 0 ]] && ok "DO_NOT_TRACK: no request" || no "DO_NOT_TRACK: no request"
  nocheck "  no mark either" exists "$(state counted)"
  machine
  first_install --yes --no-count
  [[ "$(count_hits)" == 0 ]] && ok "--no-count: no request" || no "--no-count: no request"
  machine
  SANDBOX_ENV=(NEMO_NO_COUNT=1)
  first_install --yes
  SANDBOX_ENV=()
  [[ "$(count_hits)" == 0 ]] && ok "NEMO_NO_COUNT: no request" || no "NEMO_NO_COUNT: no request"
}
# the release's archive as tools/dist.sh makes it, from what this run built; signed by the given key
make_archive() { # folder version key
  local out=$1 v=$2 key=$3 n="nemo-$2-x86_64-linux" a="$WORK/arch" m
  rm -rf "$a" "$out"
  mkdir -p "$a/$n/bin" "$a/$n/modules" "$out"
  for m in "${BINS[@]}" nemo-greet; do install -m755 "$WORK/bins/$m" "$a/$n/bin/$m"; done
  for m in nemo-files nemo-view nemo-play; do mkdir -p "$a/$n/modules/$m"; cp -a "$SRC/$m/config" "$a/$n/modules/$m/"; done
  mkdir -p "$a/$n/modules/nemo-agents"; cp -a "$SRC/nemo/shell/agents/config" "$a/$n/modules/nemo-agents/"
  mkdir -p "$a/$n/modules/nemo-shot" "$a/$n/modules/nemo-greet"
  cp -a "$SRC/nemo-greet/fonts" "$SRC/nemo-greet/greet-session" "$SRC/nemo-greet/install-system.sh" "$a/$n/modules/nemo-greet/"
  tar -C "$a" -czf "$out/$n.tar.gz" "$n"
  (cd "$out" && sha256sum "$n.tar.gz" >SHA256SUMS)
  ssh-keygen -q -Y sign -f "$key" -n nemo-release "$out/SHA256SUMS" </dev/null
}
t_prebuilt() { # a release with ready binaries: no compiler, signature first, every failure stops before a change
  reset_world
  machine
  # the full install leaves every binary in the shared target, the archive takes them from there
  first_install --yes --full --from-source
  rm -rf "$WORK/bins"; mkdir -p "$WORK/bins"
  cp -- "$H"/.local/bin/nemo-* "$WORK/target/release/nemo-greet" "$WORK/bins/"
  local rel="$WORK/release"
  make_archive "$rel" 3.0.0 "$WORK/key"
  machine
  : >"$(log_for "$SCEN")"
  : >"$H/root.log"
  SANDBOX_ENV=(NEMO_PREBUILT_URL="$rel")
  check "the first install from the ready binaries" first_install --yes
  check "  the shell is in place" is_ver nemo-shell 3.0.0
  nocheck "  nothing was compiled" grep -q 'Compiling' "$(log_for "$SCEN")"
  nocheck "  no compiler was asked for" sh -c "grep 'root: .*pacman -S' '$H/root.log' | grep -qE ' (rust|gcc|pkgconf)( |$)'"
  check "  the record of what it came from" contains "$(state built/nemo-shell)" "prebuilt-3.0.0"
  check "  niri accepts the config" niri_ok
  check "modules from the same archive" apply "files view shot"
  check "  nemo-files" is_ver nemo-files 3.0.0
  check "  its .desktop" exists "$(data applications/nemo-files.desktop)"
  nocheck "  still nothing compiled" grep -q 'Compiling' "$(log_for "$SCEN")"
  : >"$H/root.log"
  check "the login screen too" apply "files login"
  check "  its root half from the archive" contains "$H/root.log" "/.local/share/nemo/modules/nemo-greet/install-system.sh --system"
  check "  and the binary, next to its script for later" exists "$H/.local/share/nemo/modules/nemo-greet/nemo-greet"
  # an update to a release that has an archive: the same way, no compiler
  publish 3.1.0
  make_archive "$rel" 3.1.0 "$WORK/key"
  : >"$(log_for "$SCEN")"
  check "an update takes the ready binaries" update
  check "  the record says 3.1.0" test "$(installed_version)" = 3.1.0
  check "  marked as from the archive" contains "$(state built/nemo-shell)" "prebuilt-3.1.0"
  nocheck "  nothing was compiled" grep -q 'Compiling' "$(log_for "$SCEN")"
  # a step that leaves a service running must not hold the run (nemo-agents --reload in a real session)
  rm -rf "$WORK/bins-keep"; cp -a "$WORK/bins" "$WORK/bins-keep"
  printf '#!/bin/sh\n[ "$1" = --reload ] && { sleep 600 & }\nexit 0\n' >"$WORK/bins/nemo-agents"
  make_archive "$rel" 3.1.0 "$WORK/key"
  SANDBOX_ENV=(NEMO_PREBUILT_URL="$rel" NIRI_SOCKET=/nonexistent)
  check "a service left behind by a step does not hold the run" sandbox timeout -k 5 240 bash -c 'out=$(bash "$1" --lang en --update 2>&1); printf "%s\n" "$out" >/dev/null' _ "$H/.local/share/nemo/src/install.sh"
  check "  and the step did start it" exists "$H/.local/bin/nemo-agents"
  command pkill -f '[s]leep 600' 2>/dev/null
  rm -rf "$WORK/bins"; mv "$WORK/bins-keep" "$WORK/bins"
  make_archive "$rel" 3.1.0 "$WORK/key"
  # what must not get through. A fresh machine each time: nothing may land
  local bad="$WORK/bad"
  rm -rf "$bad"; cp -a "$rel" "$bad"
  printf 'x' >>"$bad/nemo-3.1.0-x86_64-linux.tar.gz"
  SANDBOX_ENV=(NEMO_PREBUILT_URL="$bad")
  machine
  nocheck "a changed archive is refused" first_install --yes
  nocheck "  no shell landed" exists "$H/.local/bin/nemo-shell"
  rm -rf "$bad"; cp -a "$rel" "$bad"; rm -f "$bad/SHA256SUMS.sig"
  machine
  nocheck "an unsigned release is refused" first_install --yes
  nocheck "  no shell landed" exists "$H/.local/bin/nemo-shell"
  rm -rf "$bad"; cp -a "$rel" "$bad"
  rm -f "$bad/SHA256SUMS.sig"
  ssh-keygen -q -Y sign -f "$WORK/rogue" -n nemo-release "$bad/SHA256SUMS" </dev/null
  machine
  nocheck "a stranger's signature is refused" first_install --yes
  nocheck "  no shell landed" exists "$H/.local/bin/nemo-shell"
  # a correctly signed list that does not name the archive (another release's list, say)
  rm -rf "$bad"; cp -a "$rel" "$bad"
  sed -i 's/nemo-3\.1\.0-x86_64-linux\.tar\.gz/nemo-3.0.0-x86_64-linux.tar.gz/' "$bad/SHA256SUMS"
  rm -f "$bad/SHA256SUMS.sig"
  ssh-keygen -q -Y sign -f "$WORK/key" -n nemo-release "$bad/SHA256SUMS" </dev/null
  machine
  nocheck "an archive its signed list does not name is refused" first_install --yes
  nocheck "  no shell landed" exists "$H/.local/bin/nemo-shell"
  SANDBOX_ENV=()
}
# ── the release itself: tools/release.sh against the fake GitHub ────────────
# gh as the release needs it: repositories are bare ones in $GH_REPOS, releases folders in $GH_STORE
# (a «.draft» file while a draft). Publishing a release whose tag is not on GitHub yet fails, as it would
make_gh() {
  mkdir -p "$WORK/rel-shims"
  cat >"$WORK/rel-shims/gh" <<'SH'
#!/bin/bash
echo "gh $*" >>"$GH_STORE.log"
mkdir -p "$GH_STORE"
tag_on_gh() { [[ -n "$(git -C "$GH_REPOS/nemo.git" tag -l "$1")" ]]; }
files() { # what is left after the options
  while (($#)); do
    case $1 in --title | --notes | --target | --json | --jq | --description) shift ;; --*) ;; *) printf '%s\n' "$1" ;; esac
    shift
  done
}
case "$1 $2" in
  "auth status") exit 0 ;;
  "repo view") [[ -d $GH_REPOS/${3#*/}.git ]] ;;
  "repo create") git init -q --bare -b main "$GH_REPOS/${3#*/}.git" ;;
  "release view")
    [[ -d $GH_STORE/$3 ]] || exit 1
    [[ $* == *--json* ]] && { if [[ -f $GH_STORE/$3/.draft ]]; then printf 'draft'; else printf 'public'; fi; printf ', %s files\n' "$(ls "$GH_STORE/$3" | wc -l)"; }
    exit 0 ;;
  "release create")
    t=$3; shift 3
    [[ $* == *--draft* ]] || [[ $t == installs ]] || tag_on_gh "$t" || { echo "tag $t is not on GitHub" >&2; exit 1; }
    mkdir -p "$GH_STORE/$t"
    [[ $* == *--draft* ]] && touch "$GH_STORE/$t/.draft"
    files "$@" | while read -r f; do cp -- "$f" "$GH_STORE/$t/"; done ;;
  "release edit")
    [[ -d $GH_STORE/$3 ]] || exit 1
    if [[ $* == *--draft=false* ]]; then tag_on_gh "$3" || { echo "tag $3 is not on GitHub" >&2; exit 1; }; rm -f "$GH_STORE/$3/.draft"; fi ;;
  "release upload")
    t=$3; shift 3
    [[ -d $GH_STORE/$t ]] || exit 1
    files "$@" | while read -r f; do cp -- "$f" "$GH_STORE/$t/"; done ;;
  *) echo "gh stub: $*" >&2; exit 1 ;;
esac
SH
  # the binaries: named as tools/dist.sh names them, a signed SHA256SUMS over them (the real build is tested
  # by tools/dist.sh itself and the «prebuilt» scenario)
  cat >"$WORK/rel-shims/dist-stub" <<'SH'
#!/bin/bash
set -e
v=$(cat VERSION) key=
while (($#)); do [[ $1 == --sign ]] && key=$2; shift; done
mkdir -p dist
printf 'nemo %s archive\n' "$v" >"dist/nemo-$v-x86_64-linux.tar.gz"
for p in shell files view play shot greet; do printf 'deb %s\n' "$p" >"dist/nemo-${p}_${v}_amd64.deb"; done
for m in files view play shot; do printf 'bin %s\n' "$m" >"dist/nemo-$m"; done
cd dist
sha256sum "nemo-$v-x86_64-linux.tar.gz" ./*.deb nemo-files nemo-view nemo-play nemo-shot >SHA256SUMS
ssh-keygen -q -Y sign -f "$key" -n nemo-release SHA256SUMS </dev/null
echo "  nemo-shell         1 KiB  glibc 2.34"
SH
  chmod +x "$WORK/rel-shims"/*
  # env.sh as next to the real modules: git and cargo take github.com/HamzaAkramv15/* from the working copies
  printf 'export GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0="url.file://%s/.insteadOf" GIT_CONFIG_VALUE_0="https://github.com/HamzaAkramv15/"\nexport CARGO_NET_GIT_FETCH_WITH_CLI=true\n' "$SRC" >"$SRC/env.sh"
}
rel() { # release.sh arguments…: the owner's run, from the working copies, against the fake GitHub
  env -i -C "$SRC/nemo" HOME="$H" USER=tester LANG=en_US.UTF-8 TERM=dumb \
    PATH="$WORK/rel-shims:$REAL_HOME/.cargo/bin:/usr/bin:/bin" \
    CARGO_HOME="$REAL_HOME/.cargo" RUSTUP_HOME="$REAL_HOME/.rustup" CARGO_TARGET_DIR="$WORK/target-dev" \
    GIT_AUTHOR_NAME=t GIT_AUTHOR_EMAIL=t@t GIT_COMMITTER_NAME=t GIT_COMMITTER_EMAIL=t@t \
    NEMO_MODULES_DIR="$SRC" NEMO_RELEASE_DIST="$WORK/rel-shims/dist-stub" NEMO_RELEASE_CHECK=true \
    GH_STORE="$WORK/gh-releases" GH_REPOS="$GH" \
    bash "$SRC/nemo/tools/release.sh" "$@" </dev/null >>"$(log_for "$SCEN")" 2>&1
}
# every HEAD and tag of the working copies: what --undo must give back exactly
refs_now() { local r; for r in nemo "${MODULES[@]}"; do printf '%s ' "$r"; git -C "$SRC/$r" for-each-ref --format='%(refname)=%(objectname)' refs/heads refs/tags | tr '\n' ' '; echo; done; }
gh_ref() { git -C "$GH/$1.git" rev-parse -q --verify "$2" 2>/dev/null; }
t_release() { # the owner's release: made here, taken back, made again, published; a machine updates to it
  reset_world
  make_gh
  rm -rf "$WORK/gh-releases" "$WORK/gh-releases.log"
  local y="$SRC/nemo" m
  g -C "$y" config user.signingkey "$WORK/key.pub"
  # the world is at 3.0.0: the changelog's newest section says so
  sed -i 's/^## 2\.7\.0$/## 3.0.0/' "$y/CHANGELOG.md" "$y/CHANGELOG.ru.md"
  g -C "$y" commit -q -am "changelog at 3.0.0" && push nemo
  # a machine on 3.0.0 with nemo-view, to update once the release is out
  machine
  first_install --yes
  check "a machine on 3.0.0 with nemo-view" apply view
  # as on the owner's machine: origin is GitHub over https and the modules have no remote yet. git reaches
  # the fake GitHub by a rule of the fake HOME that is shorter than env.sh's: were env.sh to leak into the
  # release's git, its rule would win and the release would talk to the working copies instead
  mkdir -p "$WORK/gh-root"
  ln -sfn "$GH" "$WORK/gh-root/HamzaAkramv15"
  git config --file "$H/.gitconfig" url."file://$WORK/gh-root/".insteadOf "https://github.com/"
  g -C "$y" remote set-url origin https://github.com/HamzaAkramv15/nemo.git
  for m in "${MODULES[@]}"; do git -C "$SRC/$m" remote remove origin 2>/dev/null || true; done
  # one module changed since its tag, the others not
  printf '\n' >>"$SRC/nemo-view/README.md"
  g -C "$SRC/nemo-view" commit -q -am "view: a change for 3.1.0"
  local before
  before="$(refs_now)"
  check "--check says it is ready" rel --check 3.1.0
  check "  nothing changed" test "$(refs_now)" = "$before"
  nocheck "--check refuses a version that is not newer" rel --check 2.0.0
  check "the release is made here" rel 3.1.0
  check "  its tag is signed by the release key" g -C "$y" -c gpg.ssh.allowedSignersFile="$y/keys/allowed_signers" tag -v v3.1.0
  check "  VERSION is 3.1.0" test "$(cat "$y/VERSION")" = 3.1.0
  check "  the changed module got 3.1.0" g -C "$SRC/nemo-view" -c gpg.ssh.allowedSignersFile="$y/keys/allowed_signers" tag -v v3.1.0
  check "  and its crate the version" grep -q '^version = "3.1.0"' "$SRC/nemo-view/Cargo.toml"
  nocheck "  an unchanged one did not" g -C "$SRC/nemo-files" rev-parse -q --verify refs/tags/v3.1.0
  check "  modules.lock names both" sh -c "grep -qx 'nemo-view v3.1.0' '$y/modules.lock' && grep -qx 'nemo-files v3.0.0' '$y/modules.lock'"
  check "  the flake takes the new tag" grep -q 'github:HamzaAkramv15/nemo-view/v3.1.0' "$y/flake.nix"
  check "  flake.lock pins it with its hash" sh -c "grep -A12 '\"nemo-view\": {' '$y/flake.lock' | grep -q '\"ref\": \"v3.1.0\"' && grep -c narHash '$y/flake.lock' | grep -qx 6"
  check "  PKGBUILD carries the archive's sum" grep -q "sha256sums=('$(sha256sum "$y/dist/nemo-3.1.0-x86_64-linux.tar.gz" | cut -d' ' -f1)')" "$y/packaging/arch/PKGBUILD"
  check "  the ebuild and its Manifest" sh -c "test -f '$y/packaging/gentoo/nemo-bin-3.1.0.ebuild' && grep -q '^DIST nemo-3.1.0-x86_64-linux.tar.gz ' '$y/packaging/gentoo/Manifest'"
  check "  the spec" grep -q '^Version: *3.1.0$' "$y/packaging/fedora/nemo.spec"
  check "  the release commit has the archive's time" test "$(g -C "$y" log -1 --format=%ct v3.1.0)" -gt 0
  nocheck "  nothing reached GitHub" sh -c "git -C '$GH/nemo.git' tag -l v3.1.0 | grep -q . || git -C '$GH/nemo-view.git' tag -l v3.1.0 | grep -q ."
  check "  --status runs" rel --status 3.1.0
  check "taken back with --undo" rel --undo 3.1.0
  check "  every HEAD and tag as before" test "$(refs_now)" = "$before"
  check "  the tree is clean" test -z "$(git -C "$y" status --porcelain --untracked-files=no)"
  check "made again" rel 3.1.0
  # the first time a module has no repository on GitHub at all: --push makes it
  local niri_tag
  niri_tag="$(gh_ref nemo-niri refs/tags/v3.0.0)"
  rm -rf "$GH/nemo-niri.git"
  check "published with --push" rel --push 3.1.0
  check "  a module new to GitHub got its repository, main and tag" sh -c "grep -q 'gh repo create HamzaAkramv15/nemo-niri' '$WORK/gh-releases.log' && test '$(g -C "$SRC/nemo-niri" rev-parse refs/tags/v3.0.0)' = '$niri_tag' && git -C '$GH/nemo-niri.git' rev-parse -q --verify refs/heads/main >/dev/null && test \"\$(git -C '$GH/nemo-niri.git' rev-parse refs/tags/v3.0.0)\" = '$niri_tag'"
  check "  every module knows its GitHub now" sh -c "for m in ${MODULES[*]}; do git -C '$SRC'/\$m remote get-url origin | grep -qx \"https://github.com/HamzaAkramv15/\$m.git\" || exit 1; done"
  check "  the module's tag is on GitHub" test "$(gh_ref nemo-view refs/tags/v3.1.0)" = "$(g -C "$SRC/nemo-view" rev-parse refs/tags/v3.1.0)"
  check "  and its main" test "$(gh_ref nemo-view refs/heads/main)" = "$(g -C "$SRC/nemo-view" rev-parse 'v3.1.0^{commit}')"
  check "  the shell's tag and main" test "$(gh_ref nemo refs/heads/main)" = "$(g -C "$y" rev-parse 'v3.1.0^{commit}')"
  check "  the release is public with its binaries" sh -c "test ! -e '$WORK/gh-releases/v3.1.0/.draft' && test -s '$WORK/gh-releases/v3.1.0/SHA256SUMS.sig' && test -s '$WORK/gh-releases/v3.1.0/nemo-3.1.0-x86_64-linux.tar.gz'"
  check "  it was a draft until the tag was there" sh -c "grep -n 'release create v3.1.0 --draft' '$WORK/gh-releases.log' | grep -q ."
  nocheck "  --undo refuses a published release" rel --undo 3.1.0
  # the published release, installed as any user gets it
  check "the machine updates to it" update
  check "  the shell is 3.1.0" is_ver nemo-shell 3.1.0
  check "  nemo-view is 3.1.0" is_ver nemo-view 3.1.0
  check "  niri accepts the config" niri_ok
  # the next one changes nemo-px, whose tag is public by now: it never moves, a new patch version comes
  printf '\n// a change for 3.2.0\n' >>"$y/shell/px/src/lib.rs"
  # 3.1.0 took «Unreleased» with it: the next release has its own lines
  perl -0pi -e 's/^## Unreleased\n\n/## Unreleased\n\n- nemo-px: a change.\n\n/m' "$y/CHANGELOG.md"
  perl -0pi -e 's/^## Не выпущено\n\n/## Не выпущено\n\n- nemo-px: изменение.\n\n/m' "$y/CHANGELOG.ru.md"
  g -C "$y" commit -q -am "px: a change for 3.2.0"
  before="$(refs_now)"
  check "a release with a changed nemo-px" rel 3.2.0
  check "  nemo-px gets the next patch version, signed" g -C "$y" -c gpg.ssh.allowedSignersFile="$y/keys/allowed_signers" tag -v px-v3.0.1
  check "  the public px tag stays where it was" test "$(g -C "$y" rev-parse refs/tags/px-v3.0.0)" = "$(gh_ref nemo refs/tags/px-v3.0.0)"
  for m in nemo-files nemo-view nemo-play nemo-shot nemo-greet; do
    check "  $m takes it and gets 3.2.0" sh -c "grep -q 'tag = \"px-v3.0.1\"' '$SRC/$m/Cargo.toml' && git -C '$SRC/$m' rev-parse -q --verify refs/tags/v3.2.0 >/dev/null && grep -qx '$m v3.2.0' '$y/modules.lock'"
  done
  check "  nemo-niri (no nemo-px) keeps its tag" grep -qx 'nemo-niri v3.0.0' "$y/modules.lock"
  check "taken back with --undo" rel --undo 3.2.0
  check "  every HEAD and tag as before" test "$(refs_now)" = "$before"
}
t_askpass() { # the first-run setup's way in: no terminal, the password through SUDO_ASKPASS
  reset_world
  machine
  first_install --yes
  : >"$H/root.log"
  SANDBOX_ENV=(SUDO_ASKPASS=/usr/bin/false)
  check "the first-run setup's command" apply "files login" "gtk qt"
  SANDBOX_ENV=()
  check "  the root shell took the askpass" contains "$H/root.log" "sudo -A bash -c"
  check "  packages went through it" contains "$H/root.log" "root: "
  check "  the greeter binary too, after the build" contains "$H/root.log" "sudo -A $H/.local/share/nemo/modules/nemo-greet/install-system.sh"
  check "  the login step came first" sh -c "grep 'install-system.sh --system' '$H/root.log' | head -1 | grep -q root:"
}
t_standalone() { # a module taken on its own, from its repository, without the shell
  reset_world
  local m
  for m in nemo-view nemo-play nemo-shot nemo-files; do
    machine
    rm -f "$SRC/$m/target"
    ln -s "$WORK/target" "$SRC/$m/target"
    check "$m: builds on its own" sandbox bash -c 'cd "$1" && cargo build --release --locked' _ "$SRC/$m"
    nocheck "  without a terminal or --yes it installs nothing" sandbox "$SRC/$m/target/release/$m" install
    nocheck "  nothing was put in" exists "$H/.local/bin/$m"
    check "  install --yes" sandbox "$SRC/$m/target/release/$m" install --yes
    check "  it runs" is_ver "$m" 3.0.0
    check "  uninstall takes it away" sandbox "$SRC/$m/target/release/$m" uninstall --yes
    nocheck "  the binary is gone" exists "$H/.local/bin/$m"
    rm -f "$SRC/$m/target"
  done
}
t_tty() { # the terminal side: a real pty, nobody at the keyboard but the Enter below
  reset_world
  machine
  local out="$LOGS/tty.typescript"
  # Enter after the first screen is up: a key pressed earlier is dropped when the menu starts reading
  check "the terminal install finishes in a pty" sandbox timeout -k 5 900 bash -c "(sleep 4; printf '\n') | TERM=xterm-256color script -qec 'bash \"$SRC/nemo/install.sh\" --lang en' '$out'"
  check "  it drew the bar and the percent" grep -q '█.*%' "$out"
  check "  and the first screen" grep -q 'I will install only the base' "$out"
  check "  the cursor came back" grep -q $'\e\[?25h' "$out"
  check "  the base is there" exists "$H/.local/bin/nemo-shell"
}
t_migrate() { # the real previous release (2.7.0, one repository) updates to this one through its own updater
  reset_world
  machine
  local d="$SRC/nemo-old"
  rm -rf "$d"; mkdir -p "$d"
  git -C "$ROOT" archive v2.7.0 | tar -xC "$d"
  # the old release signed by the test key, on the same history as the new one
  printf 'test namespaces="git" %s\n' "$(cut -d' ' -f1,2 "$WORK/key.pub")" >"$d/keys/allowed_signers"
  rm -rf "$GH/nemo.git"; git init -q --bare -b main "$GH/nemo.git"; git -C "$GH/nemo.git" config uploadpack.allowFilter true
  git init -q -b main "$d"; git -C "$d" remote add origin "$GH/nemo.git"
  px_tags
  commit_tag "$d" v2.7.0
  g -C "$d" push -q origin main --tags
  sandbox bash -c 'cat "$1" | bash -s -- "${@:2}"' _ "$d/install.sh" --lang en --yes --no-sudo --no-monocraft --no-fork --no-login >>"$(log_for "$SCEN")" 2>&1
  check "2.7.0 is installed" test "$(installed_version)" = 2.7.0
  check "  with its own apps" exists "$H/.local/bin/nemo-files"
  user_mark
  printf '%s\n' "ScreenCast=nemo;" >/dev/null
  # the new tree on top of the old history, as a release
  (cd "$SRC/nemo" && git ls-files -z --cached --others --exclude-standard | tar --null -T - -cf -) | tar -xC "$d"
  git -C "$d" ls-files -z | while IFS= read -r -d '' f; do [[ -e $SRC/nemo/$f ]] || rm -f "$d/$f"; done
  printf '3.1.0\n' >"$d/VERSION"
  shell_ver "$d" 3.1.0
  local m
  for m in "${MODULES[@]}"; do module_tag "$m" 3.1.0; done
  : >"$d/modules.lock"
  for m in "${MODULES[@]}"; do printf '%s v3.1.0\n' "$m" >>"$d/modules.lock"; done
  commit_tag "$d" v3.1.0
  g -C "$d" push -q origin main --tags
  check "the old updater takes the new release" update
  check "  the record says 3.1.0" test "$(installed_version)" = 3.1.0
  check "  the shell is 3.1.0" is_ver nemo-shell 3.1.0
  check "  its modules came over" is_ver nemo-files 3.1.0
  check "  the new modules came in" is_ver nemo-shot 3.1.0
  check "  and the services" is_ver nemo-agents 3.1.0
  check "  settings and user.kdl kept" user_kept
  check "  niri accepts the config" niri_ok
  check "  screen sharing moved to nemo-agents" contains "$(cfg xdg-desktop-portal/niri-portals.conf)" "ScreenCast=nemo-agents"
  nocheck "  the embedded theme script is gone" exists "$(state theme.py)"
  check "  a retired group is not recorded" sh -c "! grep -q 'drives' '$(state install.json)'"
  # it says what is missing in this bare sandbox (no session): exit 1 is a report, anything above a crash
  check "  nemo doctor runs" sh -c "'$H/.local/bin/nemo-shell' doctor >/dev/null 2>&1; [ \$? -le 1 ]"
}

SCENARIOS=(base prebuilt release pieces standalone module-files module-view module-play module-shot module-login module-games module-fork full failures repair privacy askpass tty migrate)
run_scenario() {
  SCEN=$1
  : >"$(log_for "$SCEN")"
  say "$SCEN"
  case $1 in
    base) t_base ;;
    prebuilt) t_prebuilt ;;
    release) t_release ;;
    pieces) t_pieces ;;
    standalone) t_standalone ;;
    module-*) t_module "${1#module-}" ;;
    full) t_full ;;
    failures) t_failures ;;
    repair) t_repair ;;
    privacy) t_privacy ;;
    askpass) t_askpass ;;
    tty) t_tty ;;
    migrate) t_migrate ;;
    *) echo "unknown scenario: $1 (--list)" >&2; exit 2 ;;
  esac
}

[[ ${1:-} == --list ]] && { printf '%s\n' "${SCENARIOS[@]}"; exit 0; }
list=("$@")
((${#list[@]})) || list=("${SCENARIOS[@]}")
mkdir -p "$WORK" "$LOGS"
world
for s in "${list[@]}"; do run_scenario "$s"; done
printf '\n%s checks passed, %s failed\n' "$PASS" "$FAIL"
((FAIL)) && { printf '  %s\n' "${FAILED_NAMES[@]}"; exit 1; }
exit 0

# Validation matrix

What has been checked, how, and what could not be. Last updated 2026-11-20 on one machine: a laptop with one internal
display (1920×1080 @ 144 Hz), Fedora 45, niri 26.04-nemo.6, integrated graphics only. Nothing here is a claim about
hardware that was not available.

**Status words.** *Tested here*: done by hand or by script on this machine, result recorded. *Unit-tested*: a test in
the repository covers the logic, not the real environment. *Not tested*: not done; the reason and the manual procedure
are given.

## Crash and recovery

| Scenario | Status | How | Result |
|---|---|---|---|
| Shell killed (`kill -9`) | Tested here | killed the running shell, timed until it answered again | back in 1.4 s (niri's start loop) |
| Agents killed (`kill -9`) | Tested here | killed `nemo-agents`, waited 90 s | **failed**: never restarted until the shell restarted. Fixed: the shell checks the service's lock every 20 s and starts it again. Re-tested: back after 18 s. Unit-tested lock logic. |
| Coffee (caffeine) across a shell restart | Tested here | switched on, restarted the shell, checked the logind lock and the state file | kept; unit-tested encode/decode and deadlines |
| Compositor crash and recovery | Not tested | would end the session | manual: `pkill -x niri` from a TTY and check the login screen returns |
| Failed application launch | Not tested | | manual: launch a `.desktop` whose `Exec` is missing from the launcher and the dock, shell must stay up |
| Logout and login | Not tested | would end the session | manual after the next boot |

## Updates and signatures

| Scenario | Status | How | Result |
|---|---|---|---|
| Release signed by the trusted key | Unit-tested with real keys | temporary git repository, `ssh-keygen`, `git verify-tag` | `good:<name>` |
| Signed by a key that is not in the list | Unit-tested with real keys | same | `bad` (a valid signature is not enough) |
| Unsigned tag | Unit-tested with real keys | same | `unsigned` |
| Tag changed after signing | Unit-tested with real keys | rewrote the tag message, re-hashed | does not verify as good |
| No keys known | Unit-tested | | `nokeys` |
| Install despite a failed signature | Built, not tried by hand | log line format unit-tested; the UI says whether the signature is bad or missing; the terminal asks to type INSTALL | decision is appended to `~/.local/state/nemo/update-overrides.log` |
| Published release downloads | Tested here | downloaded `SHA256SUMS` and its signature from the public 3.2.0 release, verified against `keys/allowed_signers` | signature OK; assets listed: archive, six `.deb`, loose binaries |
| Install from the downloaded archive on a clean machine | Not tested | no clean machine | manual: fresh VM, run the one-line installer, then `nemo doctor` |
| Failed download, corrupted package, interrupted install | Not tested | | manual: disconnect the network during `nemo update apply`; the previous version must stay usable |
| Rollback | Not tested in this pass | | manual: `nemo update rollback` after an update, then `nemo update status` |
| GitHub problem reports | Unit-tested on sample data | | not run against the real repository |
| Fedora "install the held ones too" | Not tested | would change system packages | manual in a VM: `nemo-fedora-update --apply --include-held` |

## Files

| Scenario | Status | How | Result |
|---|---|---|---|
| Copy by kernel and by hand, big file | Unit-tested | existing test (40 MB) | passes |
| Replace a file keeps the old one in the trash | Unit-tested | `a_replaced_file_goes_to_the_trash_whole` with its own trash | old content in the trash, new in place |
| Cancel removes the half-written file | Read in the code (audit), not exercised by a test in this pass | | |
| Disk full | Not tested | needs a small filesystem | manual: mount a 2 MB tmpfs and copy a larger file into it; the error must be shown and no partial file left |
| Permission errors | Not tested in this pass | | manual: copy into a read-only folder |
| Network disconnect during a transfer | Not tested | | manual: copy to a mounted network share and cut the link |
| Folder with 30,300 entries | Tested here | opened in the real window, measured | about a second, 0.2 s of CPU, 17-23 MB, no CPU when idle |
| Batch rename: patterns, conflicts, swapping two names, rollback on a failure in the middle | Unit-tested on real files | `nemo-files/src/batch.rs` | all names put back, no content lost; not tried by hand in the window |
| Type filter, archive contents, storage view | Unit-tested; the pictures were rendered and looked at | `kinds.rs`, `archive.rs` (a real tar through `bsdtar`), `storage.rs` | not tried by hand with the keyboard |
| Pause and resume, cancel while paused | Unit-tested | a 200 MB copy stood still while paused and finished after resuming; cancel removed the half file | not tried by hand with the button |
| Idle memory of an empty window | Tested here | `/proc` | 7.7 MB (was 15.1 MB) |

## Widgets

| Scenario | Status | How | Result |
|---|---|---|---|
| Every widget kind at small and large size | Unit-tested by pictures | 16 extra golden pictures (`tools/golden.sh`), pixel-identical across the registry and option-schema refactors | unchanged |
| Option schema (panel, menu, click cycling) | Unit-tested | `widgetkind.rs` tests: options and order per kind, cycling and wrap | passes |
| Layouts: parse, positions, save/load, backup round trip | Unit-tested | `layouts.rs` tests | passes |
| Applying, restoring and saving a layout through the settings window | Verified (tools/e2e.sh layouts) | the real shell in a nested niri: apply Reef, save, put it back; the e2e run found that saved layouts went to `~/.config/layouts` (fixed) | `tools/e2e.sh layouts` |
| Cost of widgets | Tested here | nested fixtures 0 / 1 / 3 / 8 widgets | PSS 8.9 / 9.4 / 9.7 / 10.6 MB, anonymous memory flat at 3.1-3.2 MB, 4 wakeups/s in every case |
| Layout on a different resolution or a second monitor | Not tested | fractions of the monitor are unit-tested; no second display | manual: apply a layout, change the scale in settings → screens |

## Panels, bar and island

| Scenario | Status | How | Result |
|---|---|---|---|
| Widget panel styles (glass, frost, solid, pixel, outline, glow) and trims | Tested here by pictures | rendered offscreen and zoomed 3× on a bright and a dark background; `tools/widget-audit.sh` (39 cards) in the gate | clean corners, no stray pixels, no content outside cards |
| Changing panel style, trim, roundness, border, shadow or opacity while the desktop runs | Verified in a nested niri (tools/e2e.sh panels); not dragged by hand on the live desktop | clicks and slider values go through the shell's own path and the saved settings are checked | `tools/e2e.sh panels` |
| Island pill and card follow the corner style | Tested here by pictures | `bar:island-*`, `notif:island*` | pixel chamfer in a notched theme |
| Island closing and morphing animation | **Not seen in motion** | logic and pictures only; a frame-by-frame capture of the nested session is polluted by niri's own screenshot notifications | manual |
| Split bar | Tested here by pictures | `bar:split` | three plates; hotplug, a real second monitor and a crowded bar are Not tested |
| Widget gallery and the battery and shortcuts widgets | Tested here by pictures | `settings:widgets-gallery`, `widget:battery-*`, `widget:quick-*` | the shortcuts tiles are not clicked by test |
| Fractional widget placement after a resolution change | Not tested | needs a second resolution; the fractions are unit-tested through layouts | manual: change the scale in settings → screens |
| Keyboard backlight goes out when idle and comes back | **Not tested live**: the laptop was on its charger (policy default is battery only); the decision table is unit-tested | measured saving: about 1.5 W (paired runs) | manual: unplug, wait 20 s, press a key |

## Battery and idle cost

| Scenario | Status | How | Result |
|---|---|---|---|
| Idle cost of the shell (fixtures `minimal`, `realistic`) | Tested here | `tools/budget.sh`: a fresh shell in a nested niri, throwaway home, 35 s after start | 9.0 / 9.8 MB PSS, 5-8 wakeups/s, 0.1-0.4% CPU, 19 threads |
| Idle cost of the live shell | Tested here (report only) | `tools/budget.sh --live` | 12.3 MB, ≈ 12 wakeups/s, 0.3% CPU with two script widgets on; varies with wallpaper and widgets |
| Where the battery power goes | Tested here | RAPL package vs battery, 30 s legs, repeated; powertop 2.15 | package 3.7-5.2 W of 10.4-12.8 W at the battery |
| Refresh rate 144 Hz vs 60 Hz | Tested here | paired legs on battery-minus-package | −0.59 W at 60 Hz (3 of 3 pairs); whole-battery comparison under a live browser was inconclusive |
| Keyboard backlight on vs off | Tested here | interleaved A/B/C, three rounds | −1.5 W off (−2.0 W with powertop tunables, −18%) |
| Powertop tunables | Tested here (runtime only, reverted) | same | −0.48 W |
| Long run (8 h) and login-to-usable time | Not tested | | roadmap Phase 0/1 follow-up |
| Suspend/resume and lid power | Not tested | would suspend the session | manual |

## Display, graphics and laptop

| Scenario | Status | Reason |
|---|---|---|
| Multiple monitors, hotplug | Not tested | one display only. Manual: connect a second display, check the bar, dock and wallpaper on both, unplug it |
| Different refresh rates, fractional scaling | Not tested | one display (144 Hz at scale 1). Manual: set 1.25 and 1.5 scale in settings → screens |
| Hybrid NVIDIA | Not tested | integrated graphics only |
| Fullscreen games, screen recording and sharing | Not tested | manual: share a screen from a browser and check the picker and the indicator |
| Suspend, resume, lid close | Not tested | would suspend the session; manual: close the lid and open it, check the lock screen, night light and caffeine |
| Battery and power profile transitions | Unit-tested | the policy logic; not tried unplugging |
| Night-light schedule | Unit-tested | not watched over a full day |

## Installation

| Scenario | Status | Result |
|---|---|---|
| Update install from this tree on this machine | Tested here | many times; the latest left the shell answering and the settings intact |
| Migration from the old name | Tested here | settings, themes and the niri include line carried over; the old commands and launchers removed afterwards |
| Installer dry runs | Tested here | part of `tools/check.sh` |
| Fresh install on an empty home | Not tested | manual in a VM |
| Uninstall and leftover detection | Not tested | dangerous on the host. Manual in a VM: `./install.sh --uninstall`, then `find ~ -name '*nemo*'` |
| Login screen (greetd, nemo-greet) | Installed, not seen running | takes over at the next boot; the start script falls back to the text login if the pixel one fails |

## Findings recorded so far

| # | Level | Finding | State |
|---|---|---|---|
| 1 | P1 | `nemo-agents` was not restarted when it died | fixed and re-tested |
| 2 | P1 | Open-ended caffeine was forgotten at every shell restart | fixed and tested |
| 3 | P1 | After the rename, a config marked `// YAMI` was treated as the user's own, so fastfetch kept an old short config | fixed |
| 4 | P1 | The rename damaged credit lines (the upstream was called "nemo") in the READMEs and in every language | fixed |
| 5 | P2 | Break-glass install left no record and did not say whether the signature was bad or missing | fixed (log, wording, typed confirmation) |
| 6 | P2 | The test suite writes into the real state folder (for example the update keys file) | fixed 2026-11-22: `tools/test.sh` runs the tests in a throwaway home and checks the real update-keys file did not change; the gate uses it |
| 7 | P2 | The window title shows "Nemo 0.1.0" while the release is 3.2.0 | open: two numbering schemes |
| 8 | P2 | No screen-reader support: the interface is drawn by hand and exposes nothing to assistive tools | open, large |
| 9 | P2 | Arabic is about 40% translated and layouts are not mirrored | open |
| 10 | P2 | README pictures were Yami's author's desktop | fixed: `tools/readme-shots-offline.sh` renders Nemo's own from the real drawing code |
| 11 | P2 | Opening the launcher re-read every `.desktop` file; animated surfaces redrew at the display's full rate (144 Hz) | fixed: cached by folder time, capped near 70 fps; 88 to 50 ms CPU per open/close, measured |
| 12 | P2 | No comparison of rendered pictures | fixed: `tools/golden.sh` in the gate, 134 pictures |
| 13 | note | Files has an editable path, filter and search already; an old audit note said they did not | corrected in `nemo-files/AUDIT.md` |
| 14 | P2 | Settings had no place for health and no short description per tab; deep tabs mixed with everyday ones | fixed: five groups, descriptions, «essentials only», a status tab |
| 15 | P2 | Files lacked batch rename, a type filter, archive contents, pause and a storage view | added with tests |
| 16 | P2 | The idle budget measured the live desktop, so it failed or passed for reasons unrelated to the code (a bigger wallpaper, more widgets, a build running) | fixed 2026-11-22: `tools/budget.sh` measures a fresh shell in a nested niri with a throwaway home (fixtures `minimal` and `realistic`); `--live` reports the live shell without failing |
| 17 | P3 | `tools/golden.sh --update` could record pictures from a stale binary | fixed 2026-11-22: `--update` rebuilds first; a comparison warns when the sources are newer than the binary |
| 18 | P2 | On battery Nemo changes nothing by default (the two profile rules are empty); no visible power report | open: roadmap Phase 1 |
| 19 | P1 | Battery draw (ASUS TUF FX507VV, 2026-11-22): the CPU package is ≈ 35% of the battery draw; the keyboard backlight alone is ≈ 1.5 W (≈ 14%), the 144 Hz panel ≈ 0.6 W, powertop's platform tunables ≈ 0.5 W (each measured as paired runs, reverted afterwards); the shell's own CPU use is not a main cause (0.3% CPU, 12 wakeups/s) | open: Nemo has no policy for any of them yet (roadmap Phase 1) |

### End-to-end checks (tools/e2e.sh)

Run against the real shell in a nested niri with a throwaway home (test input only with `NEMO_TEST_IPC=1`). Results of the first run: panels 7/7, shortcut tiles 9/9 (do-not-disturb flips and flips back, coffee writes its state file), layouts 10/10 after fixing the save folder, gallery 1/1, split bar 5/5, island 6/6 after fixing the closing redraw (the pill stayed at full width until it vanished). Not covered: the keyboard backlight (needs the real LED), fractional placement after a resolution change, a live colour picture of the bar on the real desktop.

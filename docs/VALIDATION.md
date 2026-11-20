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
| Idle memory of an empty window | Tested here | `/proc` | 7.7 MB (was 15.1 MB) |

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
| 6 | P2 | The test suite writes into the real state folder (for example the update keys file) | open: tests should use a throwaway home |
| 7 | P2 | The window title shows "Nemo 0.1.0" while the release is 3.2.0 | open: two numbering schemes |
| 8 | P2 | No screen-reader support: the interface is drawn by hand and exposes nothing to assistive tools | open, large |
| 9 | P2 | Arabic is about 40% translated and layouts are not mirrored | open |
| 10 | P2 | README pictures were Yami's author's desktop | fixed: `tools/readme-shots-offline.sh` renders Nemo's own from the real drawing code |
| 11 | P2 | Opening the launcher re-read every `.desktop` file; animated surfaces redrew at the display's full rate (144 Hz) | fixed: cached by folder time, capped near 70 fps; 88 to 50 ms CPU per open/close, measured |
| 12 | P2 | No comparison of rendered pictures | fixed: `tools/golden.sh` in the gate, 134 pictures |
| 13 | note | Files has an editable path, filter and search already; an old audit note said they did not | corrected in `nemo-files/AUDIT.md` |

# Configuration reference

English · [Русский](CONFIG.ru.md)

Everything the settings window changes is a key in a plain JSON file; editing the file by hand
works the same and is picked up live. A key with a bad value falls back to its default (the rest
of the file is kept), unknown keys are ignored: `nemo-shell config check` lists both.

| File | Owner | Edit by hand |
|---|---|---|
| `~/.config/nemo/shell.json` | the shell; nemo-files, nemo-view and `nemo recolor` read the look from it | yes, live |
| `~/.config/nemo/files.json` | nemo-files | yes (read at start) |
| `~/.config/nemo/style.json`, `style.<module>.json` | the look of every module (the shell writes it from settings → look) | yes |
| `~/.config/nemo/scripts/` | **you**: bar modules and widgets from scripts | yes |
| `~/.config/nemo/layouts/` | **you**: widget layouts (`*.json`), saved by settings → widgets → layouts → save the current one | yes |
| `~/.config/nemo/themes/`, `templates/` | **you**: saved themes, recolouring templates | yes |
| `~/.config/niri/config.kdl` | **you**: Nemo only adds one include line at its top (`nemo niri connect`), what you write below it wins | yes |
| `~/.config/nemo/niri/nemo.kdl` | the shell: what that line brings, the parts below that are on in settings → system → niri | no, rewritten |
| `~/.config/nemo/niri/base.kdl` | the shell: nemo's niri look (input, layout, animations, window rules, environment) | no, rewritten |
| `~/.config/nemo/niri/binds.kdl` | the shell (settings → keys) | no, rewritten |
| `~/.config/nemo/niri/rules.kdl` | the shell (windows, look, games) | no, rewritten |
| `~/.config/nemo/niri/colors.kdl`, `cursor.kdl` | `nemo recolor` | no, rewritten |
| `~/.config/nemo/niri/outputs.kdl` | the shell (settings → screens) | no, rewritten |
| `/var/lib/nemo-greet/look/` | the shell: what the login screen is drawn from (`look.json`: palette, language, monitors, layouts; `wall.png`: the wallpaper), yours since the install | no, rewritten |
| `/var/lib/nemo-greet/log/` | the login screen: `greet.log` (its starts, niri's messages), `last` (how the last start went) | no, read with `nemo login status` / `log` |
| `~/.local/state/nemo/palette.json` | the shell: the derived palette other apps are coloured from | no |
| `~/.config/xdg-desktop-portal/niri-portals.conf` | the shell (settings → services); kept as is once it no longer starts with `# nemo:` | through the settings |
| `~/.local/state/nemo/install.json` | the installer and `tools/deploy.sh`: version, commit, source, components | no |
| `~/.local/state/nemo/update.json` | the last update check | no |

## The terminal

`nemo` is the same binary as `nemo-shell` (a link in `~/.local/bin`): without a command it
prints the help instead of starting the shell. `nemo help <command>` explains one command.

```sh
nemo open settings:updates                # a panel: launcher clipboard settings[:tab] power lock walls screenshot cal sound media net bt sys profile tray weather privacy
nemo palette | nemo palette ash | nemo palette '#3a9fd0' | nemo palette wallpaper
nemo palette show | nemo palette export NAME   # every colour with its hex (the {{names}} of templates); the palette as a saved theme
nemo wallpaper ~/Pictures/x.png           # no path: the palette's pixel wallpaper
nemo restart                              # restart the shell (refuses while a game has focus; --force)
nemo caffeine [on|off|toggle|MINUTES|Nh]            # caffeine: no idle lock, no dimming, no sleep by itself, for N minutes or until you turn it off
nemo night [on|off|toggle|always|schedule|temp K]   # night light: switch it, keep it on all day (always) or back on its schedule, set the temperature (2500-6000 K); saved in shell.json, so it survives restarts and reboots
nemo status                               # shell, portal, niri, last update check
nemo doctor                               # programs, D-Bus services, config: what is missing and what it costs
nemo doctor games                         # ntsync, gamemode, MangoHud: what is there, one command for the rest
nemo report [-o FILE]                     # versions, doctor, this boot's journal, configs in a tar.gz for an issue (secrets scrubbed)
nemo setup [STEP]                         # the first-run setup again: language, look, bar, keys, niri, modules (STEP: niri, modules…)
nemo ocr FILE                             # text from an image, the same recognition as «text» in the screenshot overlay (needs tesseract)
nemo theme export FILE | import FILE|URL  # the look as one file: palette, effects, fonts, bar, widgets, wallpaper (never commands); an imported theme is kept in your themes, a URL must be https
nemo theme save [NAME] | list | apply NAME | remove NAME   # your themes in ~/.config/nemo/themes; `Nemo` is built in (the pixel/normal balance, keeps your palette and wallpaper) (settings → color → themes); save with no name names it after the palette
nemo templates [force NAME]                # render your templates and print what came out of each (force overwrites one changed by hand)
nemo icons [NAME | auto | add PATH]        # icon sets: list them, switch (auto: the system's), install your own from an archive or a folder into ~/.local/share/icons
nemo cursor [NAME | Nemo | system | size N | add PATH]   # cursors: the Nemo pixel one, any installed set, system (leave alone); the size; your own set
nemo export FILE | import FILE [--yes]    # every setting as one file; niri/user.kdl, custom keys and bar scripts only with --yes; before an import the current state goes to ~/.local/state/nemo/before-import-*.json (undo: nemo import that file --yes)
nemo login sync                           # give the login screen the shell's look now (the shell does it on every change)
nemo login status | log | preview         # the login screen: installed, greetd enabled, its last start; its log; it over the desktop (password nemo or Esc)
nemo clean [--dry-run]                    # remove the build cache (nemo-niri sources, the shell's target: gigabytes); --force also drops a tree with your patch work
nemo version                              # every part with the installed commit
nemo update [check|apply|list|to X.Y.Z|rollback|status|log|need]
nemo msg <command>                        # raw shell command (nemo help msg)
nemo msg launcher TEXT                    # the launcher with the text typed in: ?setting, /file, >command (for a key)
nemo-shell config list                    # every key = value; * marks what differs from the default
nemo-shell config get dockPins
nemo-shell config set dockSide '"left"'   # VALUE is JSON; a bare word is taken as a string
nemo-shell config set animSpeed 0.7
nemo-shell config reset dockSide          # one key back to its default
nemo-shell config reset                   # the whole file (the old one is kept as .json.bak)
nemo-shell config check                   # bad values and unknown keys
nemo-shell config kdl                     # what the shell writes to ~/.config/nemo/niri/rules.kdl
nemo-shell config lite-auto               # lite mode on if this machine is weak or virtual (the installer runs it)
nemo-shell binds list | print | write     # keybindings
```

## The look of every module

Every module reads `~/.config/nemo/style.json`; the Nemo shell writes that file from its settings
(settings → look), without the shell it is yours. `~/.config/nemo/style.<module>.json` (`files`
`view` `play` `shot` `agents` `setup`) holds only what one module does differently. Without any of
them a module is neutral and follows the system's light or dark theme. `NEMO_STYLE` names another
file for one run. The files are reread when they change.

| Key | Values | Meaning |
|---|---|---|
| `preset` | `plain`, `nemo` | the base: neutral or the pixel look; everything below overrides it |
| `accent`, `palette` | `#rrggbb`, a preset id | the colour every tone is derived from |
| `light` | `true`, `false` | absent: as the system |
| `corners` | `notch`, `round`, `square` | panel and button corners |
| `radius` | pixels | panel corner radius with `round` |
| `shadow` | `hard`, `soft`, `none` | shadow under panels |
| `reveal` | `fade`, `none`, `dither`, `scan`, `noise`, `wipe`, `blocks` | how panels appear |
| `font` | a font family | empty: `sans` with `plain`, Tiny5 with `nemo` |
| `iconPixel`, `iconTheme` | `true`, `false`; a theme name | app icons |
| `animations`, `animSpeed` | `true`, `false`; a number | on or off, speed |
| `wallpaperCmd` | a command with `{}` | «set as wallpaper» outside the Nemo shell, e.g. `swww img {}` |
| `lang` | `ru`, `en`, `pt-BR`, `es`, `de`, `fr`, `uk`, `pl` | empty: as the system |
| `colors` | an object of `#rrggbb` | any of `bg` `panel` `surface` `surfaceHi` `codeBg` `border` `dim` `accent` `deep` `glow` `bright` `selection` `text` `textDim` `textFaint` `onAccent` `textHi` `ok` `warn` |

## shell.json

### Colour and look

| Key | Default | Meaning |
|---|---|---|
| `profileBattery`, `profileAc` | `""` | power profile to set on battery and on AC: `power-saver`, `balanced`, `performance`; empty leaves it alone |
| `weekNumbers` | `false` | week numbers (ISO 8601) in the calendar |
| `caffeineAuto` | `false` | turn the coffee on while a window is fullscreen, off when it leaves |
| `notifMuted` | `""` | apps whose notifications skip the popup and go to history (comma separated) |
| `island` | `false` | notifications and volume popups as pills at the top centre under the bar |
| `personality` | `"quiet"` | the fish speaks on charger, caffeine and update events: `off` (never), `quiet` (system events only), `playful` (also a hello when you unlock the screen and a word when you clear all notifications). Each event has two or three phrases that take turns |
| `palette` | `"nemo"` | preset: `nemo` (lavender on graphite) `blood` `ash` `sakura` `amber` `venom` `ice` `dusk` |
| `accent` | `""` | own accent `#rrggbb`; empty: the preset's |
| `preset` | `"nemo"` | the style of every module: `nemo` (pixel) or `plain` (neutral), [details](#the-look-of-every-module) |
| `corners`, `shadow` | `""`, `"soft"` | `notch` `round` `square`; `hard` `soft` `none`; empty: from the style |
| `radius` | `-1` | corner radius in pixels; `-1`: from the style |
| `colors` | `{}` | your colours over those derived from the accent |
| `tint` | `1.0` | how strongly the graphite takes the accent's hue |
| `glass` | `0.78` | panel opacity, 0.15 (glass) … 1 (solid); lower values are raised to 0.15, where panels would be all but invisible |
| `lift` | `0.0` | panel brightness: 0 near-black graphite, 1 noticeably lighter |
| `inner` | `1.0` | opacity of cards, graphs and fields inside panels |
| `dim` | `0.5` | dim behind menus and settings |
| `blurShell`, `blurTerm` | `true` | niri blur under the shell's panels / terminals |
| `iconStyle` | `"normal"` | the icons of the shell's panels, bar and control panel: `""` follows the style (pixel for the pixel look, normal for the neutral one), `pixel` the 7×7 bitmaps, `normal` smooth vector icons (about forty are drawn; the rest stay pixel). Settings → look |
| `blurXray` | `true` | niri's default: blur only the wallpaper behind a panel, so an app window behind it stays sharp (cheap). `false` blurs everything behind panels and terminals, windows too, so a panel looks the same with or without an app behind it (heavier for the GPU). Settings → look |
| `blurStrength` | `3` | how strong that blur is, 1–8 (niri `passes` and `offset`: 1 is a light haze, 8 is frosted glass and costs the GPU more); 3 is what it was before this setting. Settings → look |
| `animations` | `true` | Bayer dissolve, panel motion; off everywhere when false |
| `lite` | `false` | lite mode for weak and virtual machines (old laptops, VMs, software rendering): no blur, no shell or niri animations, opaque inactive windows; the settings above stay as they are and come back when it is off. A fresh install turns it on by itself on a VM, without GPU acceleration, on pre-2011 Intel graphics or with ≤ 4 GB and ≤ 4 threads (`config lite-auto`); `nemo doctor` says why |
| `animSpeed` | `1.0` | duration multiplier: 0.5 faster, 2 slower |
| `light` | `false` | light theme: paper instead of graphite, the accent deepened until it reads; `nemo recolor` switches GTK (adw-gtk3, `prefer-light`), Qt, KDE, kitty, btop, the cursor and the light icon variant with it |
| `reveal` | `"dither"` | how panels appear: `dither` (Bayer 4×4), `scan` (interlaced lines), `noise`, `wipe` (diagonal with a dithered edge), `blocks` (Bayer, double cells) |
| `revealGrain` | `1` | cell size of the reveal, 1–4 |
| `animClose` | `true` | panels close with the reveal backwards; off: they vanish at once |
| `anims` | `{}` | each group of panels its own animation (settings → animations): keys `windows`, `popups`, `menus`, `notifs`, `widgets`, `dock`, each `{"open": "", "close": "", "speed": 1.0, "motion": true}`; `open`/`close` = a `reveal` pattern or `none`, empty = as everywhere |
| `scanlines` | `false` | every second line of the shell's panels a little darker, like an old monitor |
| `wallpaper` | `""` | path to your own image (PNG, JPEG, WebP); empty: the generated pixel wallpaper |
| `wallDir` | `""` | the folder the wallpaper picker shows as a grid (settings → wallpaper → pick, `nemo open walls`; set by settings → wallpaper → folder, by a folder typed into the path field or by the picker's «folder…» button); empty: `~/Pictures/Wallpapers` (or `wallpapers`), else the folder of the current wallpaper, else `~/Pictures`. Thumbnails share nemo-files' cache `~/.cache/nemo/thumbs` |
| `wallFit` | `"fill"` | `fill`: cover the screen, crop the rest; `fit`: the whole image, margins in the background colour |
| `wallPixel` | `1` | pixel cells of your image: 1 as is, 2…8 → cells of 2, 3, 4, 6, 8, 12, 16 px |
| `wallTone` | `""` | colouring of your image: `""` as is, `tint` the accent's shades, `palette` palette tones with Bayer dithering |
| `wallDim` | `0.0` | darken the wallpaper towards the background colour, 0…0.8 |
| `wallAccent` | `false` | take the accent from your image, again with every new one; the whole palette follows it (background, panels, borders, the 16 terminal colours, every recoloured program and template). Settings → color → «palette from the wallpaper» or `nemo palette wallpaper`; picking a colour by hand (also `nemo config set accent`) turns it off |
| `wallScheme` | `tonal` | how the wallpaper's colour becomes the accent: `tonal`, `vibrant` (the most saturated hue), `content` (as in the picture), `muted`, `mono` |
| `templatesOff` | `[]` | your templates that are switched off (file names in `~/.config/nemo/templates`) |
| `themeGtk4` | `true` | GTK 4 / libadwaita apps (Nautilus, GNOME apps) in the palette too; off: their own look, the rest of GTK still recoloured |
| `wallKanji` | `true` | the 闇 glyph in the wallpaper corner |
| `wallSkwd` | `false` | the wallpaper comes from [skwd-wall](https://github.com/liixini/skwd-wall): the shell reads what it writes (`~/.cache/skwd-wall-v2/last-wallpaper.json`, `colors.json`; `SKWD_WALL_V2_CACHE` and `paths.cache` respected) and changes nothing in its config. On a monitor where skwd-wall draws (pictures by skwd-paper, videos and scenes by skwd-wall-vk, transitions; seen from its niri layer) the shell hides its own wallpaper; otherwise (skwd-wall's pick-only mode) the shell draws the picked image itself in its cells and tones, a video or scene by its preview frame. The lock and login screens show the same picture. `rules.kdl` gets the layer rule that puts skwd-paper's overview backdrop behind the workspaces |
| `wallSkwdColors` | `true` | with `wallSkwd`: the accent is skwd-wall's `primary` and the theme follows its background (light or dark), so a palette made in its theme designer recolours the whole rice |
| `iconTheme` | `""` | the icon set (settings → look → app icons, `nemo icons`): the shell and nemo's apps use it, and so do GTK and Qt when they are restyled; empty: the one the system already uses |
| `iconPixel` | `true` | app icons as clean pixel art: the icon is drawn large, averaged down to a grid of 16–30 cells (2 px cells in the dock), in 5–10 of its own colours with a dark outline, no dithering |
| `iconTint` | `false` | pixel icons in four tones of the palette instead of their own colours |
| `fetchFull` | `false` | fastfetch with the hardware (kernel, packages, CPU, GPU, memory, disk, display); off: who, OS, wm, shell, terminal, uptime |
| `lang` | `""` | `ru`, `en`, `pt-BR`, `es`, `de`, `fr`, `uk`, `pl`, or empty to follow `LC_ALL`, `LC_MESSAGES`, then `$LANG`; locale names such as `pt_BR.UTF-8`, `pt` or `de_AT` are also accepted; a line not translated yet shows in English |
| `themeApps` | all of them | the other programs `nemo recolor` recolours (settings → color → apps, the installer's `--restyle`): `gtk` (gtk.css, settings.ini, gsettings), `qt` (qt6ct, the KDE colour scheme, kdeglobals), `kitty` (its colours and font; off takes the `include nemo.conf` line out of `kitty.conf`), `fastfetch` (its `config.jsonc` while that is nemo's; your own one is kept and nemo's look goes to `nemo.jsonc` next to it), `btop`, `firefox` (menus), `mangohud` (only a config nemo wrote), `foot` (`foot/nemo-colors.ini` and an `include=~/…` line at the end of `foot.ini` under its own `[main]`, so it wins over colours set above; new windows), `cava` (a gradient in the `[color]` section of its config, open cava reloads at once), `starship` (a `[palettes.nemo]` table that also retints `red`, `green` and the other names, and `palette = "nemo"` unless you chose another palette). A program left out is not touched. Up to 2.6 it was `appThemes` (foot, cava and starship only), read once and carried over |

### Apps in the palette colours

niri's borders, the login screen and your templates always follow the palette; GTK, Qt and KDE,
kitty, fastfetch, btop, Firefox-family menus, MangoHud, foot, cava and starship do when they are
in `themeApps` (all of them unless you untick some). For any other
app write a template: a file in `~/.config/nemo/templates` that is the app's colour config with
`{{accent}}`, `{{bg}}`, `{{text}}`, `{{color0}}`…`{{color15}}` and the other palette names
(`.strip` drops the `#`, `.rgb` gives `r, g, b`), plus a line saying where it goes and, if the app
can reload, whom to signal. Every palette change renders it again, and so does any change in the
templates folder (the shell looks every 5 s). It is written only under `~/.config`,
`~/.local/share` or `~/.cache`, never over a file you changed by hand, and not at all while it
names a colour the palette does not have (a typo would break the app's config); files ending in
`.md` are skipped.

Ready templates come with the shell for Ghostty, Alacritty, WezTerm, Helix, Neovim, zathura, tmux,
fuzzel, Vesktop and Equibop (settings → color → apps → «ready templates»): «add» puts a copy among
your templates, and the app needs one line of its own config that the list shows (`theme = nemo`,
`include nemo`…). Each writes only its own file next to the app's config.

Settings → color → apps lists what came out of each template: written, changed by hand (an
«overwrite» button renders it over your edit and keeps that as `*.bak-nemo`), an unknown colour, no
`nemo-output` line or a target outside those folders. In a terminal, `nemo templates` renders them
and prints the same, `nemo templates force NAME` overwrites one; the record is
`~/.local/state/nemo/restyle.json`.

```ini
# nemo-output: ~/.config/mako/config
# nemo-reload: USR1 mako
background-color={{bg}}
border-color={{accent}}
text-color={{text}}
```

### Fonts

Settings → look → «pixel look» moves these keys, `iconPixel` and `cursor` (unless it names a cursor set of your own) together in four steps: smooth (the system sans and cursor everywhere), softer (pixel shell panels only), Nemo only (GTK and Qt apps smooth, the installer's default) and all pixel.

| Key | Default | Meaning |
|---|---|---|
| `fontShell` | `"sans"` | the shell's font (any fontconfig family); `"sans"` the system sans, empty: Tiny5 (the pixel look). Non-pixel fonts are sized to Tiny5's cap height so layouts hold |
| `fontFiles`, `fontView` | `"sans"` | nemo-files / nemo-view; empty: as the shell |
| `fontUi` | `"sans"` | GTK/Qt apps: empty Tiny5 on its grid, `"sans"` the system sans, or a family |
| `fontPt` | `0` | size of a non-pixel `fontUi`, pt; 0: 11 |
| `fontSize` | `0` | size of the pixel `fontUi`: 16, 24, 32 px, or 0 by screen height |
| `terminalFish` | `true` | kitty starts fish when it is installed (suggestions from history as you type, highlighting); only the terminal, your login shell stays. `false`: kitty starts your usual shell. Settings → fonts → terminal |
| `fontMono` | `""` | terminal (kitty) and Qt fixed font; empty: Monocraft, else Terminess |
| `browserMenus` | `true` | Firefox-family browsers: right-click menus, dropdowns and tooltips in the palette (nothing else of the browser is touched) |
| `browserFont` | `false` | and the pixel font in the browser's menus and toolbars (never on web pages) |

### Bar

| Key | Default | Meaning |
|---|---|---|
| `barLeft`, `barCenter`, `barRight` | see `config list` | module ids in order: `launcher` `workspaces` `mode` `tasks` `media` `tray` `net` `netspeed` `bt` `kb` `cpu` `ram` `temp` `gpu` `disk` `weather` `bat` `vol` `profile` `night` `caffeine` `privacy` `clock` `notif` `settings` `power` `sep` (`bat`: laptop battery, hidden without one; `gpu`: graphics load from amdgpu or `nvidia-smi` every 3 s, hidden without either, «sleep» while a laptop's discrete card sleeps; `disk`: how full `/` is; `privacy`: lights up while a program records the microphone, holds the camera or shares the screen, and is hidden otherwise; a click says who and can mute the microphone or stop the sharing) |
| `cpuStyle` | `"gauge"` | `gauge` `graph` `number`, for `cpu` `ram` `gpu` `disk` (`disk` has no history: `graph` draws a gauge) |
| `tempStyle` | `"full"` | the `temp` module: `full` like `cpu` and `ram` (label and `cpuStyle`), `compact` a small thermometer column and the number |
| `trayMode` | `"icons"` | `icons`: every tray icon on the bar; `arrow`: one arrow that opens them as a list (clicks work as on the bar; a dot when an app asks for attention). A network applet (`nm-applet` and the like) is left out while the bar has `net` |
| `trayPixel` | `false` | tray icons (on the bar and in the arrow's list) redrawn in pixel cells like the dock and the launcher with `iconPixel`: own colours, or palette tones with `iconTint` (#19) |
| `batHideFull` | `false` | the `bat` module hides while the laptop is on power and not charging: full, or held at a charge limit (#21) |
| `controlTiles` | `["wifi", "bt", "dnd", "night", "mic", "theme"]` | the tiles of the control panel (right-click an empty spot on the bar, `nemo msg popup control`), in order, up to 6: `wifi` `bt` `dnd` `night` `mic` `mute` `theme` `curtain` `settings` `lock` `power`. The panel also has the clock on the wallpaper (click: the wallpaper picker), upright volume and brightness sliders, what is playing (with back, play and next) and a bar row: the screen edge in one click and «bar settings» |
| `privacyMic`, `privacyCam`, `privacyScreen` | `true` | what the `privacy` module watches: the microphone (PulseAudio/PipeWire recordings that are not of a sound output), the camera (who holds `/dev/video*`), screen sharing (niri's casts; the shell's own screenshots do not count); watched only while the module is on the bar |
| `mediaViz` | `"bars"` | the equalizer of the `media` module and the music popup: `bars` `line` `blocks` `wave` `dots`, or `off` (no equalizer, and no audio capture for it) |
| `mediaView` | `"cover"` | what the music popup shows (the button in its header switches it): `cover` (cover, buttons, equalizer), `both` (cover and lyrics), `lyrics`, `minimal` (title and buttons) |
| `clockSeconds`, `clockDate` | `false`, `true` | |
| `clock12` | `false` | 12-hour clock with AM/PM (`3:05 PM`; the hour has no leading zero) in the bar, side bar, control panel, calendar popup, clock widget and lock screen. Settings → bar → clock and calendar. The login screen follows once nemo-greet reads it |
| `dateStyle` | `"dm"` | the short date in the bar: `dm` (`Tu 17.11`), `md` (`Tu 11/17`), `iso` (`2026-11-17`) |
| `hijri` | `false` | show the Hijri date in the calendar popup (tabular calendar; may differ by a day from the announced one) |
| `arabicDigits` | `false` | draw all digits as Eastern Arabic (٠١٢٣) |
| `weekend` | `"satsun"` | which days the calendar marks as the weekend: `satsun` (Saturday, Sunday) or `frisat` (Friday, Saturday) |
| `weekStart` | `"mon"` | the first day of the week in the calendar popup and widget: `mon`, `sun`, `sat` |
| `taskWidth` | `190` | width of a task button, px (60–400); `tasks` works in any group and is the one module that narrows when the bar runs out of room |
| `barTaskGap` | `4` | gap between task buttons, px (0–16) |
| `barSep` | `"dots"` | separator look: `dots` `line` `blocks` `gap` |
| `barSepW`, `barSepColor` | `10`, `"border"` | separator width, px (2–32); colour `border` `accent` `dim` |
| `barGap` | `8` | gap between modules, px (0–32) |
| `barPadL`, `barPadR` | `6`, `6` | from the left and right edge of the bar to the outermost modules, px (0–64); on a side bar, from the top and the bottom |
| `barMid` | `12` | from the side modules to the window list (and the centre modules), px (0–64) |
| `barFloat` | `false` | the bar as a separate panel: `barMargin` from the screen edges, a border all round, a hard shadow; windows keep clear of it |
| `barWidthPc` | `100` | bar length, % of the monitor (30–100): its width, or its height on a side bar; below 100 the panel is centred (`barFloat` off: it hangs from its edge) |
| `barMargin` | `6` | gap between a floating bar and the screen edges, px (0–32) |
| `barOutputs` | `[]` | the screens with a bar, by niri name (`["eDP-1"]`; `niri msg outputs`); empty: every screen. When none of them is connected (a laptop away from its monitor) every screen gets one. Settings → screens → «bar on this screen» |
| `barCenterAlign` | `"center"` | `barCenter` modules: `center` at the middle of the bar (a long left group only pushes them as far as needed), `after-left` right after the left group (the windows of `tasks` sat there before 2.7) |
| `barPos` | `"top"` | screen edge with the bar: `top`, `bottom`, `left` or `right` (settings → bar → look: click an edge on the screen map). A side bar is a column: the left modules on top, the centre ones in the middle, the right ones at the bottom, each drawn narrow (the clock as hours over minutes, numbers without units, icons instead of labels, windows as icons). Popups, tray and window menus, the OSD, the dock and widgets keep clear of the bar at any edge |

### Windows, dock, screenshots, system

`niriLook`, `niriKeys`, `niriRules`, `niriColors`, `niriScreens` (default `true`): which parts of the niri configuration Nemo writes into `nemo.kdl` (niri look, keys, window rules, colours and cursor, outputs). A part you switch off is left out, so your own `config.kdl` decides it. `barV` (internal): the version of the bar layout, used once to migrate old separators; leave it alone.

`highContrast` (default `false`): high contrast over any palette: text at least 10:1 (second text 7:1, faint 4.5:1), accent and status colours 4.5:1, borders 3:1 against the surfaces, opaque panels, no frosting. Applies to the shell and to every module that reads `style.json` after it is rebuilt against the new nemo-px. `reduceMotion` (default `false`): no animations anywhere (the same as `animations: false`, kept separate so that turning it off restores your own choice). Settings → accessibility has both. `dndSchedule` (default `false`), `dndFrom` (default `22`) and `dndTo` (default `7`): quiet hours for do not disturb, from the hour `dndFrom` up to (not including) `dndTo`, past midnight too; `dndFrom` equal to `dndTo` is an empty window. The manual `dnd` switch always wins.

| Key | Default | Meaning |
|---|---|---|
| `stacking` | `false` | «desk» mode: every window floats, the shell draws title bars |
| `titlebars` | `true` | title bars and the resize corner in desk mode. Desk mode focuses a window on click (and raises it); focus follows the mouse only in tiling |
| `floatMedia`, `floatPip` | `true` | photo/video viewers and picture-in-picture open floating |
| `floatApps` | `""` | more app-ids to open floating, comma separated |
| `castHide` | `""` | app-ids that viewers of a screen share never see: niri paints them black in the stream (`block-out-from "screencast"`), on your screen they stay as they are; screenshots still show them. Settings → windows, or `nemo msg cast-hide` and a click on the window |
| `floatW`, `floatH` | `0.62`, `0.78` | their size, share of the screen |
| `winGap` | `8` | gap between tiled windows, px (0–64): niri `gaps` in `rules.kdl` |
| `winLeft`, `winRight`, `winTop`, `winBottom` | `8` each | from the screen edge to the windows, px (0–128; the top one from the bar): niri `struts` = edge − gap. Struts in your niri `config.kdl` replace these whole |
| `winRadius` | `0` | window corner radius, px (0–24; 0 square, like pixels): niri `geometry-corner-radius` with `clip-to-geometry`, the border follows it. Games stay square; fullscreen and maximized windows are square anyway; in desk mode with title bars only the bottom corners of floating windows round, so the shell's title bar joins without a gap |
| `winBorder` | `2` | window border width, px (0–8; 0: none); its colours come from the palette |
| `cursor`, `cursorSize` | `"nemo"`, `24` | `nemo` (pixel, in the palette), the name of any installed cursor set (settings → look → cursor, `nemo cursor`), or `system` (Nemo leaves the cursor alone) |
| `dock` | `true` | show the dock |
| `dockSide` | `"bottom"` | `bottom` `top` `left` `right` |
| `dockHide` | `"smart"` | `never` `smart` (visible on an empty desk) `always` |
| `dockSize`, `dockAnim` | `40`, `"grow"` | icon size; hover: `grow` `wave` `lift` `none` |
| `dockBounce`, `dockLabels`, `dockRunning`, `dockLauncher`, `dockTrash` | `true` | |
| `dockBadges` | `false` | numbers and progress bars on the dock icons: what apps report themselves (Telegram, Firefox downloads, Thunderbird… over the Unity LauncherEntry interface) or, failing that, the number of that app's notifications in the history |
| `dockZoom` | `8` | how many pixels an icon grows under the cursor, 0 to 32 (0: it does not grow); the wave setting (`dockAnim`) lets the neighbours grow too |
| `dockPins` | | app ids (desktop file names) pinned to the dock |
| `curtainStyle` | `"blur"` | what stream viewers see while the screen is hidden (Mod+Shift+H), no labels: `blur` the frozen screen blurred, `pixels` in big pixels, `wall` the wallpaper |
| `curtainCell` | `24` | pixel size of `pixels` |
| `lockAfter`, `screenOffAfter` | `10`, `15` | idle minutes; 0: never |
| `lockSleep` | `true` | lock the screen before any sleep: the lid, `systemctl suspend`, the system's own timer; the shell holds a logind delay lock and lets the sleep go on once niri confirms the lock (at most 3 s) |
| `sessionRestore` | `false` | at login, start the programs that were open last time and put their first window back on its monitor and workspace (snapshot in `~/.local/state/nemo/session.json`; games stay out) |
| `night` | `false` | night light through wlsunset |
| `caffeineMinutes` | `0` | how many minutes caffeine (the coffee cup in the bar, a control-panel tile, `nemo caffeine`) lasts after a click; 0 until you turn it off. While it is on the screen is not dimmed or locked by idle and logind does not suspend the machine on its own; closing the lid still does. Settings → system |
| `fetchLogo` | `"distro"` | the fastfetch logo: `distro` (your distribution's own fastfetch logo, Fedora on Fedora, in the palette colours: the small one in the short layout, the large one in the extended layout), `pixel` (the distribution's logo icon redrawn as chunky pixels), `image` (the Nemo fish as a picture, in kitty), `blocks` (the Nemo fish as terminal half-blocks). The old value `auto` means `distro` |
| `nightTemp` | `4000` | evening colour temperature, K (2500–6000, steps of 250) |
| `nightAlways` | `false` | night light with no schedule: warm colours all day while `night` is on (wlsunset is run with the temperature and one Kelvin more, since it refuses equal ones). `nemo night always` / `schedule` |
| `nightGeo` | `""` | `"lat, lon"`: from sunset to sunrise there; `"auto"`: the same for the place the system time zone names (see `weatherCity`); empty: by the clock |
| `nightOnH`, `nightOffH` | `20`, `7` | the clock: warm from / until this hour |
| `dnd`, `notifTimeout` | `false`, `7` | do not disturb; seconds a notification stays |

### Login screen

Settings → system → login screen. The shell publishes these keys with its palette, language and
monitors to `/var/lib/nemo-greet/look`; the login screen reads them at its next start.

| Key | Default | Meaning |
|---|---|---|
| `greetWall` | `"desktop"` | its wallpaper: `desktop` the same picture as the desktop, `pixel` the palette's pixel wallpaper, `image` its own picture (`greetWallFile`), `plain` none (the theme colour) |
| `greetWallFile` | `""` | the picture for `greetWall: "image"`; choose it in settings → wallpaper → "login screen wallpaper…" (or "apply to: login" in the picker). A missing file falls back to the desktop picture |
| `greetDim` | `0.55` | the veil over the wallpaper (0.2–0.9) |
| `greetClock`, `greetHost` | `true` | the big clock and date; the machine name in the corner |
| `greetUsers` | `"list"` | `list`: the users to pick from with arrows (and «another user»); `name`: no list, the name is typed |
| `greetPower` | `true` | sleep, reboot and power off buttons (where logind or elogind allows them) |
| `greetOff` | `2` | minutes before the screens go off; 0: never |
| `greetNumlock` | `true` | numlock on |
| `greetHour` | `""` | the clock format on the login screen: empty follows the desktop (`clock12`), `12` or `24` |
| `greetNight` | `true` | night light on the login screen too, with the desktop's settings (`night`, `nightTemp`, `nightAlways`, the place or hours), when it is on there |

### Games

| Key | Default | Meaning |
|---|---|---|
| `gameFullscreen` | `true` | game windows (Steam/Proton `steam_app_*`, `gamescope`, `gameApps`) open fullscreen |
| `gameVrr` | `true` | VRR while a game is on screen (the output's VRR set to «in games», settings → screens) |
| `gameTearing` | `false` | tearing: frames shown at once, not at vblank (nemo-niri only) |
| `gameCache` | `true` | big NVIDIA/Mesa shader caches without cleanup (from the next login) |
| `gameDnd` | `true` | notifications wait in the history while a game or any fullscreen window has focus |
| `gameProfile` | `true` | «performance» power profile while a game has focus, the previous one after |
| `gameApps` | `""` | your games' app-ids, comma separated; `name*` matches the start |
| `gameNtsync` | `true` | `PROTON_USE_NTSYNC=1` for Proton when the kernel has `/dev/ntsync` (from the next login) |
| `gameGamemode` | `true` | the focused game is registered with gamemoded (no `gamemoderun %command%` needed) |

MangoHud gets a config in the palette and the pixel font (`~/.config/MangoHud/MangoHud.conf`) as
long as its first line is nemo's; delete that line and `nemo recolor` never touches it again.

### Services

What on D-Bus and in the portals Nemo does itself. Turned off, it is left to another program
(mako or dunst, waybar's tray, polkit-gnome, the GTK/GNOME portals). Notifications and the
tray change after a shell restart (`nemo restart`), portals from the next login. The polkit window,
the screen-share picker and automount belong to [nemo-agents](#nemo-agents).

| Key | Default | Meaning |
|---|---|---|
| `agentNotif` | `true` | the notification server (`org.freedesktop.Notifications`) |
| `agentTray` | `true` | the tray (`org.kde.StatusNotifierWatcher`, at most 64 items) |
| `notifMax`, `notifHistory` | `4`, `50` | cards on screen at once; how many the history keeps |
| `notifPos` | `"top-right"` | where the cards appear: `top-right`, `top-left`, `top`, `bottom-right`, `bottom-left`, `bottom` (settings → notifications: click a spot on the screen map, a sample card shows there); they keep clear of the bar and the dock |
| `osdPos` | `"bottom"` | the volume and brightness OSD: `bottom`, `top`, `center`, `left`, `right` or a corner (`top-left`, `top-right`, `bottom-left`, `bottom-right`); it keeps clear of the bar |
| `osdBright` | `true` | show the OSD when the laptop backlight changes (keys, `brightnessctl`, the firmware); a desktop monitor has no backlight and nothing runs for it |
| `notifOffset`, `osdOffset` | `10`, `64` | distance of the notification cards and of the OSD from their screen edge, px (0–400); next to the bar it is counted from the bar |
| `notifImages` | `true` | pictures in cards (icons and previews; at most 1024², files ≤ 16 MB) |
| `notifCast` | `false` | notifications show in recordings and streams; off: viewers never see them (nemo-niri ≥ `.5` simply leaves them out, other niri blacks them out) |
| `audioNames` | `{}` | your names for sound devices (outputs and microphones): PipeWire name (`pactl list short sinks`, `pactl list short sources`) → label, e.g. `{"alsa_output.usb-…-00.analog-stereo": "Headphones"}` |
| `mediaArtNet` | `true` | album covers from the player's web links (`curl`, https after redirects, ≤ 16 MB); off: only local files |
| `lyricsNet` | `true` | lyrics from lrclib.net (artist, title and length are sent) when the player has none and there is no `.lrc` or `.txt` file next to the track; asked only while the music popup shows lyrics, answers cached in `~/.cache/nemo/lyrics` |
| `btReconnect` | `true` | when Bluetooth turns on (and once per login), connect the paired and trusted headphones, speakers, keyboards and mice that are not connected yet; phones and computers connect by themselves. Settings → bluetooth |
| `weatherCity`, `weatherUnits` | `"auto"`, `"c"` | the `weather` bar module and widget: a city (`Berlin`), `latitude, longitude`, or `auto`: the city that the system time zone names (`Africa/Cairo` → Cairo), checked against the geocoder's answer for that time zone so it is the right Cairo; that one city name is all that leaves the machine. Asked from open-meteo.com every 20 minutes while one of them is shown; `f` gives °F and mph. Settings → services → network |
| `portalFiles` | `true` | file dialogs from nemo-files (else GTK) |

### Updates

`nemo update` and settings → updates work on their own clone, `~/.local/share/nemo/src` (your
working copy is never touched). A release is the newest `vX.Y.Z` tag; with `updateChannel`
`main` every commit of the branch is offered. Installing runs `install.sh --update` from the
new tree: it rebuilds from source, keeps every setting and needs no sudo. Nothing is replaced until every part has
built, and a module whose source did not change is not rebuilt. Before it starts, a rollback point goes to
`~/.local/state/nemo/snapshots` (the last three are kept, `nemo update snapshots` lists them): the programs and every
file the installer rewrites. A run that fails after it began changing things puts it all back by itself;
`nemo update rollback` goes back to the newest point (your settings, screens and niri config are left as they are now). `nemo update list` shows the last
eight releases with their signatures, `nemo update to X.Y.Z` builds and installs one of them (only
what passes `updateVerify`). Before switching to an older version the settings are copied to
`~/.local/state/nemo/settings-newer.json`; keys the older version dropped come back from it on the
next switch to a version at least as new. A switch to a whole version also rebuilds nemo-niri at
that version's level (from the next login).

An update runs without sudo. When the components chosen at install time (`shot`, `games`, …)
want packages the system lacks — a new version brought a feature with a helper program — the
installer writes them to `~/.local/state/nemo/packages-needed`; `nemo update need` prints one
`sudo` command (`--run` runs it), `nemo doctor` and settings → updates show it too, the tab with an
**install** button that opens it in a terminal. Only package names the repositories have go there.

| Key | Default | Meaning |
|---|---|---|
| `updateCheck`, `updateEvery` | `true`, `6` | check by itself at login and every N hours (only a check and a card: installing is your click) |
| `updateChannel` | `"release"` | `release` (tags) or `main` (every commit) |
| `updateVerify` | `"auto"` | `auto`: once the installed version has keys in `keys/allowed_signers`, only tags signed by them install; `require`: never unsigned; `off`: no check |
| `updateRepo` | `""` | another repository URL; empty: where the install came from |

### Keys

| Key | Default | Meaning |
|---|---|---|
| `binds` | `{}` | binding id → keys, e.g. `{"terminal": "Mod+Return", "launcher-2": ""}` (`""` turns it off). Ids: `nemo-shell binds list` |
| `customBinds` | `[]` | your commands: `[{"key": "Mod+Shift+B", "cmd": "kitty -e btop"}]` (run with `sh -c`) |
| `kbLayout` | `""` | keyboard layouts, xkb codes separated by commas (`"us,ru"`, `"de"`, `"us,ara"`); empty: the keyboard layouts and variants the system itself is set up with (`localectl`: `/etc/vconsole.conf`, `00-keyboard.conf`), else from the system languages (LANG and LC_*: English gives `us`, every other language adds its layout, non-Latin ones after `us`) |
| `kbSwitch` | `"grp:alt_shift_toggle"` | the xkb option that switches layouts (`grp:ctrl_shift_toggle`, `grp:caps_toggle`, `grp:alt_space_toggle`); empty: no switch |

Keys are written as niri does: `Mod` (Super), `Ctrl`, `Alt`, `Shift`, then a key name (`Q`,
`Slash`, `Page_Up`, `Print`, `XF86AudioMute`, `WheelScrollDown`). Anything the table cannot
express goes into your `~/.config/niri/config.kdl` below nemo's line, in plain niri syntax, and wins.

### Bar modules from scripts

An executable file in `~/.config/nemo/scripts/<name>` becomes the bar module `x:<name>` (settings
→ bar lists it with the built-in ones). The script prints a line on every update, plain text or
JSON; a script that keeps running streams lines, one that exits is run again after `interval`
seconds (10 by default). The same script can also be a desktop widget (settings → widgets →
script), where `\n` in the JSON `text` starts a new line; the bar shows the first line.

```sh
#!/bin/sh
# ~/.config/nemo/scripts/weather: a 7×7 icon, the text, a theme colour, a click, every 10 minutes
t=$(curl -fs 'https://wttr.in/?format=%t' || echo '?')
printf '{"text":"%s","icon":"temp","color":"accent","click":"xdg-open https://wttr.in","interval":600}\n' "$t"
```

Twenty ready scripts come with the shell (settings → widgets has a «ready scripts: add them all» button; `nemo scripts` lists them, `nemo scripts add NAME` or
`add all` puts a copy in your folder; yours are never overwritten): `uptime` `load` `disk`
`memtop` `cpufreq` `power` `kernel` `vpn` `docker` `flatpak` `swap` `cputemp` `ipaddr` `wifi`
`yearprogress` `utc` `processes` `fan` `gitdir` `moon`. They use no network and no account (they
only read `/proc`, `/sys` and the tools already on the machine); each is a short `sh` file you
can read and change, and the few that take a setting (`disk`, `utc`, `gitdir`) say so at the top.

`color` is a theme role: `accent` `bright` `ok` `warn` `text` `dim`. `icon` is one of the shell's
7×7 icons, e.g. `cpu` `ram` `temp` `net` `wifi` `bt` `bell` `clock` `cal` `music` `play` `power`
`bolt` `leaf` `moon` `sun` `gauge` `chart` `drop` `folder` `image` `camera` `lock` `user` `gear`.

### Integrations: actions for the launcher

A program or script tells Nemo what it can do with a manifest instead of code inside the shell. Today a manifest adds **actions to the launcher search** (type two or more letters: a title or a keyword finds it); the same file will later carry indicators and settings pages. The launcher also finds the **actions of `.desktop` files** (`[Desktop Action …]`: "New private window") and the actions of Nemo's own apps, which ship in the same format inside the shell.

A manifest is a JSON file in `~/.config/nemo/integrations/` (or `$XDG_DATA_DIRS/nemo/integrations/`):

```json
{ "api": 1, "id": "my-tool", "name": "My Tool", "requires": "my-tool",
  "actions": [ { "title": "Do it", "keywords": "go run", "icon": "gear", "command": ["my-tool", "--do-it"] } ] }
```

`command` is a list of arguments, run without a shell: nothing typed in the search ever reaches it. `requires` is a program that must be in `PATH`, otherwise the manifest stays silent. `api` must be `1`; unknown fields are ignored, so a newer manifest does not break an older shell. Limits: a title of at most 80 characters, at most 64 actions and 32 arguments, a file of at most 64 KB. A manifest with the same `id` as a built-in one replaces it. `nemo-shell integrations check` lists what was found and what is wrong.

Bar scripts have a **safe mode**: a script that fails within 3 seconds five times in a row is switched off, the bar shows `!` in its place, and `nemo scripts check` says so. `nemo scripts reset NAME` turns it back on.

### Widgets

`widgetContrast` (default `true`): widgets without a frame follow the wallpaper under them: on a light

Widget layouts: settings → desktop → widgets → **layouts** places several widgets at once (four ship with Nemo: Aurora, Reef, Minimal, Focus; yours come from `~/.config/nemo/layouts/*.json`, and a file with the name of a shipped one replaces it).
Applying one switches your other widgets off instead of deleting them and remembers the whole set and the panel look first; **put it back as it was** restores them. A layout file:
`{"name": "…", "about": "…", "about_ru": "…", "look": "plain|reef", "panel": "glass|solid|outline", "widgets": [{"kind": "sky", "size": "m", "fx": 0.70, "fy": 0.06, "frame": true}]}`;
`fx`, `fy` are the position as fractions of the monitor (so a layout fits any resolution), and a widget may also carry `source`, `style`, `anim`, `color`, `frame`, `label`, `seconds`, `date`. Widgets placed by a layout carry its id in `layout` and are replaced when another layout is applied.

`widgetPanel` (default `"glass"`): the style of widget panels: `glass`, `frost` (denser, with a sheen), `solid`, `pixel` (notched corners, a hard shadow on the pixel grid, a 2 px border), `outline` (transparent, corner brackets), `glow` (dark with a coloured halo). A widget can have its own (`panel` in its options). Every style is a complete recipe (corners, border, shadow, fill) that does not depend on the global corner and shadow settings. `widgetAlpha` (default `0`, as the style intends; 0.2 to 1): how opaque the fill is. `widgetLook` (default `"plain"`): the trim: `plain`, `accent` (a tag in the corner), `reef` (a highlighted top edge and bubbles that rise above the corner and stay for a widget without a panel), `hud` (corner sights). `widgetRadius` (default `-1` = as the style intends; 0 to 24, px): corner radius of every widget panel. `widgetBorder` (default `-1`; 0 to 4): border width. `widgetShadow` (default `-1`; 0 to 100, %): shadow strength.

`barPanel` (default `""` = the classic look of the theme): the style of the floating and split bar plates (the same names as `widgetPanel`). `barRadius` and `barBorder` (default `-1`): their corner radius and border width. `barSplitGap` (default `6`; 2 to 32, px): the smallest gap between the plates of the split bar.

`barSplit` (default `false`): split bar: three separate plates (left group, middle, right) with gaps instead of one strip; the island is the middle one. `kbdLight` (default `"idle_battery"`; `keep` or `idle`): the keyboard backlight goes out when idle (by default only on battery) and comes back with the first key; `kbdIdleSecs` (default `20`): after how many seconds. It goes through logind, no root; on an ASUS TUF the backlight drew about 1.5 W.

`widgets` is a list of `{"id", "kind", "on", "x", "y", "source", "style", "anim", "size",
"color", "frame", "label", "seconds", "date", "text"}`: `kind` is `clock` `graph` `net` `media`
`cal` `weather` `bt` `script` `note` (`bt`: the connected Bluetooth devices with their battery, a click opens settings → bluetooth); graphs take `source` `cpu` `ram` `temp` `gpu` `swap` and
`style` `bars` `line` `blocks` `wave` `dots`; a `script` widget takes the script's file name as
`source`. `x`/`y` of -1 means the default place. Easier to arrange with the
mouse: settings → widgets, edit mode.

## files.json

| Key | Default | Meaning |
|---|---|---|
| `view` | `"grid"` | `grid` or `list` |
| `zoom` | `2` | icon size in the grid, 0…4 |
| `hidden`, `foldersFirst` | `false`, `true` | |
| `sort`, `desc` | `"name"`, `false` | `name` `size` `time` `type` |
| `thumbs`, `thumbMaxMb` | `true`, `40` | thumbnails, and the largest image to thumbnail, MB. The cache `~/.cache/nemo/thumbs` (shared with the wallpaper picker) tidies itself once a day: thumbnails not opened for 120 days go, and it stays under 512 MB |
| `sidebar`, `info` | `true`, `false` | places on the left, details on the right |
| `devices` | `true` | drives, USB sticks, phones and cameras in the sidebar |
| `network` | `true` | the «network» section: mounted shares and «connect…» |
| `servers` | `[]` | addresses you connected to (`smb://nas/media`, `sftp://me@host/`), never with a password; one click mounts them again |
| `placesOrder` | `[]` | your order of the places: `home`, `desktop`, `documents`, `downloads`, `music`, `pictures`, `videos`, `trash` and bookmark paths; drag a place along the sidebar to set it |
| `placesHidden`, `devsHidden` | `[]`, `[]` | places and devices you hid (right click → hide; right click → show hidden brings them back) |
| `collapsed` | `[]` | folded sidebar sections: `places`, `devices`, `network` (a click on the section title) |
| `terminal` | `"kitty"` | «open terminal here» |
| `confirmTrash`, `singleClick`, `split` | `false` | |
| `width`, `height` | `1060`, `660` | window size at start |

Phones, cameras and network shares go through gvfs (`gio mount`): the packages `gvfs-mtp`
(Android), `gvfs-gphoto2` (cameras), `gvfs-afc` (iPhone) and `gvfs-smb` (Windows shares) add the
backends; sftp, ftp and WebDAV are in gvfs itself. A password is asked in the window and handed
to `gio` over a pipe; it is not stored. Kernel mounts (cifs, nfs, sshfs, rclone) show up too.
A phone shows in the sidebar as soon as file transfer is on (nemo-agents mounts it by
itself). An encrypted drive (LUKS, BitLocker) asks for its password on a click (`nemo-files
--unlock /dev/…` opens that window); udisks opens it, and «eject» locks it again before the
drive is powered off. A right click on a place or a drive opens it in a new tab or the other pane.

Its shortcuts are the usual ones (Ctrl+C/X/V, F2, Delete, Ctrl+T, Ctrl+L, Ctrl+H …) and match by
key position, so they work in any keyboard layout.

## nemo-play

`nemo-play [FILE|DIR|URL…]` plays every format ffmpeg knows through libmpv (the `mpv` package,
loaded at run time). One file opens its folder as a playlist in natural order. The position is
remembered in `~/.local/state/nemo/play/` and frames are saved to the Pictures folder.
Keys: space pause, ←/→ 5 s (Shift 1 s, Ctrl 30 s), ↑/↓ or the wheel volume, F fullscreen, M mute,
N/P next/previous, [ ] speed, A audio track, S subtitles, L repeat, X pixel scaling, C save a
frame, `,` `.` frame step, 1…9 jump to 10…90 %, I details, Q quit. `NEMO_PLAY_OPTS="hwdec=no,…"`
passes extra mpv options. The look (palette, language, font) is the viewer's (`fontView`).

## nemo-shot

`~/.config/nemo/shot.json`, also edited by settings → screenshots.

| Key | Default | Meaning |
|---|---|---|
| `dir` | `""` | screenshot folder; empty: Pictures/Screenshots |
| `copy`, `save`, `loupe`, `notify` | `true` | copy, save, magnifier, notification |
| `delay` | `3` | timer, seconds |
| `confirm` | `space` | `space`: select, then Space or Enter takes the shot; `instant`: the shot is taken as soon as the mouse is released (area, window), and a full-screen shot at once, without the overlay |
| `prefix` | `nemo` | the start of the file name |
| `recordAudio` | `off` | sound of screen recordings: `off`, `mic` (default microphone), `system` (what is playing) |
| `recordDir` | `""` | folder of screen recordings; empty: the Videos folder of your home (separate from the screenshot folder) |
| `recordFps` | `0` | frames per second of recordings: `0` as the screen, or `30` / `60` |

## nemo-agents

`~/.config/nemo/agents.json`, also edited by settings → services and by `nemo-agents enable|disable PART`.
`nemo-agents status` says what is on and who does the job.

| Key | Default | Meaning |
|---|---|---|
| `polkit` | `true` | the polkit password window; off: another agent |
| `screencast` | `true` | the screen-share picker and portal backend (niri); off: GNOME's window |
| `automount` | `true` | drives, memory cards, discs, phones (Android file transfer), cameras and iPhones mount by themselves when plugged in |
| `automountOpen` | `false` | open what was mounted at once; off: a notification with «open» |

"""The scenarios of tools/e2e.py. Every case drives the real shell through its own click path and then checks what it wrote."""
import json, os, subprocess, time


def open_settings(e, tab):
    """`settings TAB` toggles: if a previous case left the window open it would close it, so look and send again"""
    e.send(f"settings {tab}", 0.5)
    if "STab" not in e.zones("modal"):
        e.send(f"settings {tab}", 0.5)


def case_panels(e):
    print("panels: style, trim, roundness, opacity, border, shadow")
    open_settings(e, "widgets")
    e.click("modal", 'SCycle("widget_panels")')
    e.click("modal", 'SPanelStyle("pixel")')
    e.settle()
    e.check(e.cfg().get("widgetPanel") == "pixel", "clicking the pixel swatch sets widgetPanel=pixel")
    e.click("modal", 'SPanelLook("reef")')
    e.settle()
    e.check(e.cfg().get("widgetLook") == "reef", "clicking the reef trim sets widgetLook=reef")
    for key, v, field, want in [("widget_radius", 0.2, "widgetRadius", 4), ("widget_border", 0.4, "widgetBorder", 1), ("widget_shadow", 0.5, "widgetShadow", 50), ("widget_alpha", 1.0, "widgetAlpha", 1.0)]:
        e.send(f"t:slider {key} {v}", 0.2)
        e.settle()
        got = e.cfg().get(field)
        e.check(got == want, f"slider {key} {v} -> {field}={want} (got {got})")
    e.send("t:slider widget_radius 0.0", 0.2)
    e.settle()
    e.check(e.cfg().get("widgetRadius") == -1, "far-left roundness means -1 (as the style intends)")
    e.niri("action", "toggle-hotkey-overlay")
    e.settle(0.5)
    e.shot("panels-pixel-reef")


def case_tiles(e):
    print("shortcuts widget: tiles act on the shell")
    z = e.zones("widget:2")
    for act in ("LockNow", "NightToggle", "CaffeineToggle", "Dnd", "Power", "Settings"):
        e.check(act in z, f"the shortcuts widget has a {act} tile")
    before = e.cfg().get("dnd", False)
    e.click("widget:2", "Dnd")
    e.settle(0.8)
    e.check(e.cfg().get("dnd", False) != before, "the do-not-disturb tile flips the dnd setting")
    e.click("widget:2", "Dnd")
    e.settle(0.8)
    e.check(e.cfg().get("dnd", False) == before, "the do-not-disturb tile flips it back")
    e.click("widget:2", "CaffeineToggle")
    e.settle(0.8)
    st = os.path.join(e.HOME, ".local/state/nemo")
    files = os.listdir(st) if os.path.isdir(st) else []
    e.check(any("caffeine" in f for f in files), f"the coffee tile writes its state file ({[f for f in files if 'caff' in f] or files[:6]})")
    e.click("widget:2", "CaffeineToggle")


def case_layouts(e):
    print("layouts: apply, restore, save")
    open_settings(e, "widgets")
    e.click("modal", 'SCycle("widget_layouts")')
    orig = [(w["id"], w["kind"]) for w in e.cfg().get("widgets", []) if w.get("on")]
    e.check(len(orig) >= 2, f"two widgets are on before ({orig})")
    e.click("modal", "SLayout(1)")  # Reef
    e.settle(0.9)
    on = [w for w in e.cfg().get("widgets", []) if w.get("on")]
    e.check(on and all(w.get("layout") == "reef" for w in on), f"after applying Reef, every widget that is on comes from it ({[(w['kind'], w.get('layout')) for w in on]})")
    e.check(len(on) == 4, f"Reef puts four widgets on (got {len(on)})")
    e.check(e.cfg().get("widgetLook") == "reef", "Reef sets the reef trim")
    old_off = [w for w in e.cfg().get("widgets", []) if not w.get("layout")]
    e.check(len(old_off) >= 2 and all(not w.get("on") for w in old_off), "the old widgets are kept, switched off")
    bak = os.path.join(e.HOME, ".local/state/nemo/widgets-before-layout.json")
    e.check(os.path.exists(bak), "a backup of the previous set was written")
    e.click("modal", "SLayoutSave")
    e.settle(0.5)
    mine = os.path.join(e.HOME, ".config/nemo/layouts/my-layout-1.json")
    e.check(os.path.exists(mine), "'save the current one' writes my-layout-1.json")
    if os.path.exists(mine):
        j = json.load(open(mine))
        e.check(len(j.get("widgets", [])) == 4, "the saved layout holds the four widgets that were on")
    e.click("modal", "SLayoutRestore")
    e.settle(0.9)
    w = e.cfg().get("widgets", [])
    back = [(x["id"], x["kind"]) for x in w if x.get("on")]
    e.check(back == orig, f"'put it back as it was' restores the original widgets ({back})")
    e.check(not os.path.exists(bak), "the backup is removed after restoring")


def case_gallery(e):
    print("gallery: click a preview to add the widget")
    open_settings(e, "widgets")
    e.click("modal", 'SCycle("widget_gallery")')
    n0 = len(e.cfg().get("widgets", []))
    e.click("modal", 'SWidgetAdd("battery")')
    e.settle(0.8)
    w = e.cfg().get("widgets", [])
    e.check(len(w) == n0 + 1 and w[-1]["kind"] == "battery", f"clicking the battery preview adds a battery widget ({[x['kind'] for x in w]})")


def case_bar(e):
    print("bar: split design and plate options")
    open_settings(e, "bar")
    e.click("modal", 'SCycle("bar_page_look")')
    e.click("modal", 'SBool("bar_split")')
    e.settle(0.8)
    e.check(e.cfg().get("barSplit") is True, "the split toggle sets barSplit")
    e.click("modal", 'SCycle("bar_panel")')
    e.settle(0.5)
    e.check(e.cfg().get("barPanel") == "glass", f"the plate style cycles from classic to glass (got {e.cfg().get('barPanel')!r})")
    e.send("t:slider bar_radius 0.4", 0.2)
    e.settle()
    e.check(e.cfg().get("barRadius") == 9, f"the bar roundness slider sets barRadius=9 (got {e.cfg().get('barRadius')})")
    e.send("t:slider bar_split_gap 1.0", 0.2)
    e.settle()
    e.check(e.cfg().get("barSplitGap") == 32, "the split gap slider reaches 32")
    e.send("settings")  # close
    e.settle(0.6)
    p = e.shot("bar-split")
    e.check(os.path.exists(p), "a picture of the split bar was taken")


def case_record(e):
    print("recording: the bar shows a dot while a recording runs and the dot stops it")
    run = e.RUN
    p = subprocess.Popen(["sleep", "300"])
    pidf = os.path.join(run, "nemo-record.pid")
    open(pidf, "w").write(f"{p.pid} /tmp/e2e-fake.mp4")
    e.settle(2.5)
    z = e.zones("bar")
    e.check("RecordStop" in z, "a running recording puts the red dot in the bar")
    e.click("bar", "RecordStop")
    e.settle(3.0)
    e.check(p.poll() is not None, "clicking the dot stops the recording process")
    e.settle(2.5)
    e.check("RecordStop" not in e.zones("bar"), "the dot is gone after the recording ended")
    try:
        os.remove(pidf)
    except OSError:
        pass
    p.kill()


def case_undo(e):
    print("settings: undo walks back through changes, one at a time")
    open_settings(e, "widgets")
    e.click("modal", 'SCycle("widget_panels")')
    # start from a known state whatever the earlier cases left behind
    e.send("t:slider widget_radius 0.0", 0.2)
    e.settle(1.2)
    e.send("t:slider widget_border 0.0", 0.2)
    e.settle(1.2)
    e.send("t:slider widget_radius 0.4", 0.2)
    e.settle(1.5)
    e.send("t:slider widget_border 0.4", 0.2)
    e.settle(1.5)
    c = e.cfg()
    e.check(c.get("widgetRadius") == 9 and c.get("widgetBorder") == 1, "two changes were made")
    e.click("modal", "SettingsUndo")
    e.settle(1.2)
    c = e.cfg()
    e.check(c.get("widgetBorder") == -1 and c.get("widgetRadius") == 9, f"first undo takes back only the last change (radius {c.get('widgetRadius')}, border {c.get('widgetBorder')})")
    e.click("modal", "SettingsUndo")
    e.settle(1.2)
    c = e.cfg()
    e.check(c.get("widgetRadius") == -1, f"second undo takes back the one before (radius {c.get('widgetRadius')})")


def nav(e, what):
    e.send(f"t:nav {what}", 0.25)
    return e.out().strip()


def case_keyboard(e):
    print("keyboard: Tab walks the settings window, Enter presses, arrows move a slider")
    open_settings(e, "access")
    e.settle(0.8)
    e.send("t:tree", 0.3)
    tree = e.out()
    e.check("Toggle | high contrast" in tree, "the tree names the high contrast toggle")
    e.check("Heading | settings" in tree, "the tree has the window heading")
    seen = []
    for _ in range(40):
        f = nav(e, "next")
        seen.append(f)
        if "high contrast" in f:
            break
    e.check(any("high contrast" in f for f in seen), f"Tab reaches the high contrast toggle (visited {len(seen)} nodes)")
    e.check(len(set(seen)) == len(seen), "Tab visits each element once on the way")
    before = e.cfg().get("highContrast", False)
    nav(e, "activate")
    e.settle(1.0)
    e.check(e.cfg().get("highContrast", False) != before, "Enter flips the focused toggle")
    nav(e, "activate")
    e.settle(1.0)
    e.check(e.cfg().get("highContrast", False) == before, "Enter again flips it back")
    f = nav(e, "prev")
    e.check(f != "none" and f != seen[-1], f"Shift+Tab moves back ({f})")
    # slider: on the animations page walk to the speed slider and press right
    e.click("modal", 'STab("anim")')
    e.settle(0.8)
    before = e.cfg()
    got = None
    for _ in range(80):
        f = nav(e, "next")
        if f.startswith("Slider"):
            got = f
            break
    e.check(got is not None, f"Tab reaches a slider ({got})")
    if got:
        nav(e, "right")
        e.settle(1.2)
        after = e.cfg()
        changed = {k for k in set(before) | set(after) if before.get(k) != after.get(k)}
        e.check(bool(changed), f"the right arrow moved the slider one step (changed: {sorted(changed)})")


def case_reset(e):
    print("settings: a changed row is marked and a right-click resets only it")
    open_settings(e, "widgets")
    e.click("modal", 'SCycle("widget_panels")')
    e.send("t:slider widget_radius 0.4", 0.2)
    e.send("t:slider widget_border 0.4", 0.2)
    e.settle()
    c = e.cfg()
    e.check(c.get("widgetRadius") == 9 and c.get("widgetBorder") == 1, "two rows are changed")
    e.click("modal", 'SSlider("widget_radius"', right=True)
    e.settle(0.8)
    c = e.cfg()
    e.check(c.get("widgetRadius") == -1, f"right-click on the roundness row restores it (got {c.get('widgetRadius')})")
    e.check(c.get("widgetBorder") == 1, "the border row stays as it was")


def case_wallpapers(e):
    print("wallpapers: different pictures for the desktop and the login screen")
    from PIL import Image
    pics = os.path.join(e.HOME, "Pictures")
    os.makedirs(pics, exist_ok=True)
    for n, col in (("a-red.png", (200, 40, 40)), ("b-blue.png", (40, 60, 200))):
        Image.new("RGB", (640, 360), col).save(os.path.join(pics, n))
    e.send("walls")
    e.settle(1.2)
    z = e.zones("modal")
    e.check("WallPick(1)" in z and "WallPick(2)" in z, "the picker lists both pictures: " + " ".join(l.split(None, 5)[-1] for l in z.splitlines() if "Wall" in l))
    e.click("modal", 'SWall("target_login")')
    e.click("modal", "WallPick(2)")  # b-blue -> the login screen only
    e.settle(1.0)
    c = e.cfg()
    e.check(c.get("greetWall") == "image" and str(c.get("greetWallFile", "")).endswith("b-blue.png"), f"target login: greetWall={c.get('greetWall')} file={c.get('greetWallFile')}")
    e.check(not str(c.get("wallpaper", "")).endswith("b-blue.png"), "the desktop picture did not change")
    e.send("walls")
    e.settle(1.0)
    e.click("modal", 'SWall("target_desktop")')
    e.click("modal", "WallPick(1)")  # a-red -> the desktop only
    e.settle(1.0)
    c = e.cfg()
    e.check(str(c.get("wallpaper", "")).endswith("a-red.png"), f"target desktop: wallpaper={c.get('wallpaper')}")
    e.check(str(c.get("greetWallFile", "")).endswith("b-blue.png"), "the login picture stayed blue")
    e.send("walls")
    e.settle(1.0)
    e.click("modal", 'SWall("target_both")')
    e.click("modal", "WallPick(2)")
    e.settle(1.0)
    c = e.cfg()
    e.check(str(c.get("wallpaper", "")).endswith("b-blue.png") and str(c.get("greetWallFile", "")).endswith("b-blue.png"), "target both: both are blue")
    lw = os.path.join(os.environ.get("E2E_LOGIN_DIR", ""), "wall.png")
    e.shot("wallpapers")


def island_state(e):
    e.send("t:island", 0.02)
    r = e.out().split()
    # id, start width, current width, ms since the last draw, shown now (1/0); the pill counts only while the shell says it is shown
    return None if len(r) < 5 or r[0] == "none" or r[4] != "1" else tuple(int(x) for x in r[:4])


def sample(e, seconds, step=0.04):
    """widths of the island pill every `step` s: [(t, id, start, cur)]"""
    t0 = time.time()
    out = []
    while time.time() - t0 < seconds:
        st = island_state(e)
        if st:
            out.append((round(time.time() - t0, 2), st[0], st[1], st[2]))
        time.sleep(step)
    return out


def case_island(e):
    print("island: opening, replacing and closing animations (real notifications on the nested bus)")
    env = {**os.environ, "DBUS_SESSION_BUS_ADDRESS": e.BUS}
    notify = lambda app, title, body="", t=2500: subprocess.run(["notify-send", "-a", app, "-t", str(t), title, body], env=env, capture_output=True)
    e.send("settings")  # make sure nothing modal is open
    if e.cfg().get("island") is False:
        e.check(False, "the island is off in the test settings")
        return
    notify("Mail", "First message", "short")
    a = sample(e, 0.7)
    e.check(len(a) > 3, f"the pill is drawn after a notification ({len(a)} samples)")
    if len(a) > 3:
        widths = [x[3] for x in a]
        full = max(widths)
        e.check(widths[0] < full * 0.9 or full - widths[0] < 4, f"it opens: first width {widths[0]} grows towards {full}")
        grow = [b - a_ for a_, b in zip(widths, widths[1:])]
        e.check(all(g >= -1 for g in grow[: max(2, len(grow) // 3)]), "while opening the width never shrinks")
        first_id, first_cur = a[-1][1], a[-1][3]
        # a second notification replaces the first: the pill must flow from the old width, not snap back to 45 %
        notify("Chat", "Second message", "another one")
        b = sample(e, 0.5)
        newer = [x for x in b if x[1] != first_id]
        if newer:
            e.check(newer[0][3] >= first_cur * 0.85, f"replacing: the new pill starts at {newer[0][3]} px from the old {first_cur} px (not 45 %)")
        else:
            e.check(False, "the second notification never showed on the pill")
    # expiry: the shell keeps a notification for its own time; sample until the pill is gone and look at the tail: the width must go down
    c = sample(e, 14.0)
    e.check(c and c[-1][0] < 13.5, f"the pill goes away by itself (last sample at {c[-1][0] if c else None} s)")
    tail = [x[3] for x in c[-8:]] if c else []
    if len(tail) >= 4:
        e.check(tail[0] > tail[-1] and all(b <= a_ + 1 for a_, b in zip(tail, tail[1:])), f"closing: the width goes down to the end {tail}")
    else:
        e.check(False, f"too few samples to see the closing ({tail})")


def run(e, which):
    cases = {"panels": case_panels, "tiles": case_tiles, "layouts": case_layouts, "gallery": case_gallery, "bar": case_bar, "island": case_island, "wallpapers": case_wallpapers, "reset": case_reset, "undo": case_undo, "keyboard": case_keyboard, "record": case_record}
    e.HOME = e.HOME
    names = list(cases) if which == ["all"] else which
    for n in names:
        cases[n](e)
    print(f"\n{'all checks passed' if not e.fails else str(len(e.fails)) + ' check(s) failed'}")
    return 1 if e.fails else 0

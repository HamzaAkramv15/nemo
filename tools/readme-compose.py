#!/usr/bin/env python3
"""Copies the rendered pictures to the names the README uses and composes the desktop picture from the panels."""
import os, random, shutil, sys
from PIL import Image

src, dst = sys.argv[1], sys.argv[2]
COPY = {
    "login": "greet-demo", "files": "grid", "screenshot": "edit", "settings-color": "settings-color",
    "settings-keys": "settings-keys", "view": "view", "settings-updates": "settings-updates-new",
    "settings-agents": "settings-agents", "popup-sound": "popup-sound",
}
BG, DOT = (13, 12, 14), (28, 26, 33)


def wallpaper(w=1920, h=1080):
    """plain pixel wallpaper: the palette's dark ground with a faint dot grid and a soft light from the lower right"""
    img = Image.new("RGB", (w, h), BG)
    px = img.load()
    for y in range(0, h, 30):
        for x in range(0, w, 30):
            px[x, y] = DOT
    glow = Image.new("RGB", (w, h), (36, 31, 52))
    mask = Image.radial_gradient("L").resize((w * 2, h * 2)).crop((w // 2, h // 2, w // 2 + w, h // 2 + h))
    mask = mask.point(lambda v: max(0, 120 - v))
    return Image.composite(glow, img, mask)


def put(canvas, im, x, y):
    # с прозрачностью: пустые поля панелей не должны закрашивать обои
    canvas.alpha_composite(im, (x, y))


for lang in ("en", "ru"):
    d = os.path.join(src, lang)
    for out, name in COPY.items():
        shutil.copy(os.path.join(d, name + ".png"), os.path.join(dst, f"{out}-{lang}.png"))
    desk = wallpaper().convert("RGBA")
    ld = lambda n: Image.open(os.path.join(d, n + ".png")).convert("RGBA")
    put(desk, ld("bar"), 0, 6)
    put(desk, ld("widget-clock"), 48, 120)
    put(desk, ld("widget-cal"), 48, 290)
    put(desk, ld("widget-graph"), 1920 - 304 - 48, 120)
    put(desk, ld("widget-media"), 48, 1080 - 140 - 60)
    dock = ld("dock")
    put(desk, dock, (1920 - dock.width) // 2, 1080 - dock.height - 6)
    clean = desk.copy()
    launcher = ld("launcher")
    put(desk, launcher, (1920 - launcher.width) // 2, 230)
    desk.convert("RGB").save(os.path.join(dst, f"desktop-{lang}.png"))
    if lang == "en":
        clean.convert("RGB").save(os.path.join(dst, "clean.png"))
print("readme pictures written to", dst)

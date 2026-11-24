#!/usr/bin/env python3
"""End-to-end checks of the real shell in a nested niri (tools/e2e.sh drives it). Clicks go through the shell's own click path
(testdrive.rs, NEMO_TEST_IPC=1, nested sessions only); pictures come from grim on the nested output, so nothing touches your desktop."""
import json, os, socket, subprocess, sys, time

RUN = os.environ["E2E_RUN"]          # the nested XDG_RUNTIME_DIR
HOME = os.environ["E2E_HOME"]        # the nested HOME
WL = os.environ.get("E2E_WL", "")    # nested wayland socket name
OUT = os.environ.get("E2E_OUT", "/tmp")
BUS = os.environ.get("E2E_BUS", "")   # the nested session bus (for notify-send)
fails = []


def send(cmd, wait=0.35):
    s = socket.socket(socket.AF_UNIX)
    s.settimeout(3)
    s.connect(os.path.join(RUN, "nemo-shell.sock"))
    s.sendall((cmd + "\n").encode())
    s.close()
    time.sleep(wait)


def out():
    try:
        return open(os.path.join(RUN, "nemo-test-out.txt")).read()
    except OSError:
        return ""


def click(kind, pat, right=False):
    send(f"t:click {kind} {pat}" + (" r" if right else ""))
    r = out()
    if not r.startswith("clicked"):
        check(False, f"click {kind} {pat}: {r.strip() or 'no answer'}")
    return r.startswith("clicked")


def zones(kind):
    send(f"t:zones {kind}")
    return out()


def cfg():
    try:
        return json.load(open(os.path.join(HOME, ".config/nemo/shell.json")))
    except Exception:
        return {}


def check(ok, what):
    print(("  ok   " if ok else "  FAIL ") + what)
    if not ok:
        fails.append(what)


def niri(*a):
    sock = [f for f in os.listdir(RUN) if f.startswith("niri.") and f.endswith(".sock")]
    if sock:
        subprocess.run(["niri", "msg", *a], env={**os.environ, "NIRI_SOCKET": os.path.join(RUN, sock[0])}, capture_output=True)


def shot(name):
    p = os.path.join(OUT, name + ".png")
    subprocess.run(["grim", p], env={**os.environ, "WAYLAND_DISPLAY": os.path.join(RUN, WL)}, capture_output=True)
    return p


def settle(secs=0.6):
    time.sleep(secs)



#!/usr/bin/env python3
"""Battery benchmark: how much power the machine draws, and who draws it. No root needed.

  tools/power-bench.sh [--seconds 60] [--windows 5] [--label NAME]      measure (on battery; unplug AC first)
  tools/power-bench.sh --compare A.json B.json                          compare two measurements

Every window samples BAT*/power_now once a second, CPU idle-state residency, iGPU RC6 residency, and per-process CPU and context switches
(wakeups). A result is the mean of the window means with their spread; a difference between two results that is smaller than the spread is
reported as noise, not as a saving. One reading of power_now is an instantaneous value that swings by several watts with a live browser:
never quote one. Alternate A/B/A/B legs of at least five minutes with a quiet system and the same brightness.
Results are written to ~/.local/state/nemo/power-bench/<time>-<label>.json (never into the repository).
"""
import argparse, glob, json, os, statistics, subprocess, sys, time

HZ = os.sysconf("SC_CLK_TCK")


def read(path, default=None):
    try:
        with open(path) as f:
            return f.read().strip()
    except OSError:
        return default


def battery():
    for b in sorted(glob.glob("/sys/class/power_supply/BAT*")):
        return b
    return None


def power_w(bat):
    p = read(bat + "/power_now")
    if p and int(p) > 0:
        return int(p) / 1e6
    i, v = read(bat + "/current_now"), read(bat + "/voltage_now")
    if i and v:
        return abs(int(i)) * int(v) / 1e12
    return None


def procs():
    d = {}
    for p in glob.glob("/proc/[0-9]*"):
        try:
            comm = read(p + "/comm")
            st = read(p + "/stat").rsplit(")", 1)[1].split()
            cpu = int(st[11]) + int(st[12])
            sw = 0
            for t in glob.glob(p + "/task/*/status"):
                for line in read(t, "").splitlines():
                    if line.startswith(("voluntary_ctxt_switches", "nonvoluntary_ctxt_switches")):
                        sw += int(line.split()[1])
            d[int(p.rsplit("/", 1)[1])] = (comm, cpu, sw)
        except Exception:
            pass
    return d


def idle_states():
    r = {}
    for s in glob.glob("/sys/devices/system/cpu/cpu[0-9]*/cpuidle/state*"):
        n, t = read(s + "/name"), read(s + "/time")
        if n and t:
            r[n] = r.get(n, 0) + int(t)
    return r


def rc6():
    for f in glob.glob("/sys/class/drm/card*/gt/gt0/rc6_residency_ms"):
        return int(read(f))
    return None


def window(seconds, bat):
    ps0, id0, r0 = procs(), idle_states(), rc6()
    ncpu = len(glob.glob("/sys/devices/system/cpu/cpu[0-9]*"))
    samples, t0 = [], time.time()
    while time.time() - t0 < seconds:
        w = power_w(bat)
        if w is not None:
            samples.append(w)
        time.sleep(1)
    dt = time.time() - t0
    ps1, id1, r1 = procs(), idle_states(), rc6()
    rows = {}
    for pid, (comm, cpu, sw) in ps1.items():
        if pid in ps0:
            _, cpu0, sw0 = ps0[pid]
            r = rows.setdefault(comm, [0.0, 0.0])
            r[0] += (cpu - cpu0) / HZ / dt * 100
            r[1] += max(0, sw - sw0) / dt   # threads that exited make the sum go down: count only increases
    return {
        "power_w": statistics.mean(samples) if samples else None,
        "power_samples": len(samples),
        "idle_pct": {k: round((id1[k] - id0.get(k, 0)) / 1e6 / dt / ncpu * 100, 1) for k in id1},
        "rc6_pct": round((r1 - r0) / 10 / dt, 1) if r0 is not None and r1 is not None else None,
        "procs": {k: [round(v[0], 2), round(v[1], 1)] for k, v in rows.items() if v[0] > 0.05 or v[1] > 5},
    }


def sh(cmd):
    try:
        return subprocess.run(cmd, capture_output=True, text=True, timeout=5).stdout.strip()
    except Exception:
        return ""


def context(bat):
    mode = ""
    for line in sh(["niri", "msg", "outputs"]).splitlines():
        if "Current mode" in line:
            mode = line.strip()
            break
    bl = glob.glob("/sys/class/backlight/*")
    bright = None
    if bl:
        b, m = read(bl[0] + "/brightness"), read(bl[0] + "/max_brightness")
        bright = f"{b}/{m}" if b and m else None
    full, design = read(bat + "/energy_full"), read(bat + "/energy_full_design")
    return {
        "time": time.strftime("%Y-%m-%d %H:%M:%S"),
        "kernel": os.uname().release,
        "status": read(bat + "/status"),
        "capacity_pct": read(bat + "/capacity"),
        "health_pct": round(int(full) / int(design) * 100, 1) if full and design else None,
        "display_mode": mode,
        "brightness": bright,
        "profile": sh(["powerprofilesctl", "get"]),
        "governor": read("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor"),
        "epp": read("/sys/devices/system/cpu/cpu0/cpufreq/energy_performance_preference"),
        "nemo_shell": sh(["pgrep", "-x", "nemo-shell"]).split("\n")[0],
    }


def summarize(windows):
    pw = [w["power_w"] for w in windows if w["power_w"] is not None]
    out = {"power_mean_w": statistics.mean(pw) if pw else None,
           "power_stdev_w": statistics.stdev(pw) if len(pw) > 1 else None,
           "windows": len(windows)}
    names = {}
    for w in windows:
        for k, (c, s) in w["procs"].items():
            names.setdefault(k, []).append((c, s))
    n = len(windows)
    top = {k: [round(sum(c for c, _ in v) / n, 2), round(sum(s for _, s in v) / n, 1)] for k, v in names.items()}
    out["top"] = dict(sorted(top.items(), key=lambda kv: -(kv[1][0] * 50 + kv[1][1]))[:15])
    for key in ("rc6_pct",):
        vals = [w[key] for w in windows if w[key] is not None]
        out[key] = round(statistics.mean(vals), 1) if vals else None
    c3 = [w["idle_pct"].get("C3_ACPI", w["idle_pct"].get("C10", 0)) for w in windows]
    out["deep_idle_pct"] = round(statistics.mean(c3), 1) if c3 else None
    return out


def show(res):
    c, s = res["context"], res["summary"]
    print(f"{res['label']}  {c['time']}  {c['status']} {c['capacity_pct']}%  health {c['health_pct']}%  {c['display_mode'] or 'display ?'}  brightness {c['brightness']}  profile {c['profile']}")
    if s["power_mean_w"] is None:
        print("  power: not available (no power_now/current_now)")
    else:
        sd = f" ± {s['power_stdev_w']:.2f}" if s["power_stdev_w"] is not None else ""
        print(f"  power: {s['power_mean_w']:.2f}{sd} W over {s['windows']} window(s) of {res['seconds']} s;  deep idle {s['deep_idle_pct']}%  iGPU RC6 {s['rc6_pct']}%")
    print(f"  {'process':<24}{'cpu %':>8}{'wakeups/s':>12}")
    for k, (cpu, sw) in s["top"].items():
        print(f"  {k:<24}{cpu:>8.1f}{sw:>12.0f}")
    if c["status"] != "Discharging":
        print("  WARNING: not discharging; power is not battery draw")


def compare(a, b):
    sa, sb = a["summary"], b["summary"]
    print(f"A: {a['label']}   B: {b['label']}")
    for name, res in (("A", a), ("B", b)):
        s = res["summary"]
        sd = f" ± {s['power_stdev_w']:.2f}" if s["power_stdev_w"] is not None else " (one window: no spread)"
        print(f"  {name}: {s['power_mean_w']:.2f}{sd} W ({s['windows']} windows)")
    d = sb["power_mean_w"] - sa["power_mean_w"]
    spread = max(x for x in (sa["power_stdev_w"], sb["power_stdev_w"], 0.0) if x is not None)
    noise = 2 * spread
    print(f"  B - A = {d:+.2f} W   (noise band ±{noise:.2f} W)")
    if sa["windows"] < 3 or sb["windows"] < 3:
        print("  verdict: not enough windows (need >= 3 per leg) to call it")
    elif abs(d) <= noise:
        print("  verdict: the difference is within the noise: NOT a measured saving or cost")
    else:
        print(f"  verdict: {'B draws less' if d < 0 else 'B draws more'} by about {abs(d):.2f} W ({abs(d) / sa['power_mean_w'] * 100:.0f}%)")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--seconds", type=int, default=60)
    ap.add_argument("--windows", type=int, default=5)
    ap.add_argument("--label", default="run")
    ap.add_argument("--compare", nargs=2, metavar=("A", "B"))
    a = ap.parse_args()
    if a.compare:
        compare(*[json.load(open(p)) for p in a.compare])
        return
    bat = battery()
    if not bat:
        sys.exit("power-bench: no battery found")
    ctx = context(bat)
    if ctx["status"] != "Discharging":
        print("power-bench: warning: the battery is not discharging (unplug AC for a battery measurement)", file=sys.stderr)
    wins = []
    for i in range(a.windows):
        print(f"  window {i + 1}/{a.windows} ...", file=sys.stderr, flush=True)
        wins.append(window(a.seconds, bat))
    res = {"label": a.label, "seconds": a.seconds, "context": ctx, "windows": wins, "summary": summarize(wins)}
    d = os.path.expanduser("~/.local/state/nemo/power-bench")
    os.makedirs(d, exist_ok=True)
    path = f"{d}/{time.strftime('%Y%m%d-%H%M%S')}-{a.label}.json"
    with open(path, "w") as f:
        json.dump(res, f, indent=1)
    show(res)
    print(f"  saved: {path}")


if __name__ == "__main__":
    main()

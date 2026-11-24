#!/usr/bin/env python3
"""Tests of the decision logic of nemo-fedora-update --when-safe: no dnf, no root, no network; the machine's readings are plain data."""
import importlib.machinery
import importlib.util
import os
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
loader = importlib.machinery.SourceFileLoader("nfu", os.path.join(HERE, "nemo-fedora-update"))
spec = importlib.util.spec_from_loader("nfu", loader)
nfu = importlib.util.module_from_spec(spec)
loader.exec_module(nfu)

NOW = 1_800_000_000
DAY = 86400


def calm(**kw):
    r = {"on_battery": False, "battery": None, "game": False, "pm_running": False, "free_gb": 40.0, "snapshot": True}
    r.update(kw)
    return r


def rows():
    """classify() input: a kernel, a graphics driver, a major jump, our session, and an ordinary package"""
    return [("kernel-core", "6.20.1", "x86_64", "updates"), ("mesa-dri-drivers", "26.0.1", "x86_64", "updates"), ("pipewire", "1.6.0", "x86_64", "updates"),
            ("firefox", "150.0", "x86_64", "updates"), ("glibc", "2.45", "x86_64", "updates")]


class WhenSafe(unittest.TestCase):
    def setUp(self):
        nfu.BUILT.clear()
        nfu.installed_version = lambda name: ""  # no major-jump detection in these tests
        safe, held = nfu.classify(rows(), [])
        self.safe, self.held = safe, held
        for n in ("kernel-core", "mesa-dri-drivers", "pipewire", "firefox", "glibc"):
            nfu.BUILT[n] = NOW - 10 * DAY

    def test_the_groups_are_what_we_think(self):
        self.assertEqual(set(self.safe), {"firefox"})
        self.assertEqual(self.held["kernel-core"][2], "boot and kernel")
        self.assertEqual(self.held["pipewire"][2], "our own session")

    def test_a_calm_moment_takes_the_settled_held_groups_but_never_our_session(self):
        install, postponed, why = nfu.plan(self.safe, self.held, calm(), NOW)
        self.assertEqual(why, [])
        self.assertEqual(set(install), {"firefox", "kernel-core", "mesa-dri-drivers", "glibc"})
        self.assertIn("pipewire", postponed)
        self.assertIn("log out", postponed["pipewire"][1])

    def test_every_blocker_holds_the_held_ones_back_but_not_the_safe_set(self):
        for r, text in [(calm(on_battery=True, battery=50), "battery"), (calm(game=True), "game"), (calm(pm_running=True), "package manager"),
                        (calm(free_gb=1.0), "free"), (calm(snapshot=False), "snapshot")]:
            install, postponed, why = nfu.plan(self.safe, self.held, r, NOW)
            self.assertEqual(set(install), {"firefox"}, text)
            self.assertTrue(any(text in w for w in why), text)
            self.assertIn("kernel-core", postponed, text)

    def test_a_package_built_yesterday_waits_and_an_unknown_age_waits_too(self):
        nfu.BUILT["kernel-core"] = NOW - 1 * DAY
        del nfu.BUILT["glibc"]
        install, postponed, _ = nfu.plan(self.safe, self.held, calm(), NOW)
        self.assertNotIn("kernel-core", install)
        self.assertNotIn("glibc", install)
        self.assertIn("less than 3 days", postponed["kernel-core"][1])
        self.assertIn("mesa-dri-drivers", install, "the older one goes")

    def test_the_age_limit_can_be_changed(self):
        nfu.BUILT["kernel-core"] = NOW - 1 * DAY
        install, _, _ = nfu.plan(self.safe, self.held, calm(), NOW, min_age_days=1)
        self.assertIn("kernel-core", install)

    def test_a_major_version_jump_is_never_automatic(self):
        nfu.installed_version = lambda name: "1.0" if name == "firefox" else ""
        safe, held = nfu.classify([("firefox", "2.0", "x86_64", "updates")], [])
        install, postponed, _ = nfu.plan(safe, held, calm(), NOW)
        self.assertEqual(install, {})
        self.assertIn("major version", postponed["firefox"][0])

    def test_build_time_parses_epoch_dates_and_garbage(self):
        self.assertEqual(nfu.parse_time("1700000000"), 1700000000)
        self.assertGreater(nfu.parse_time("2026-11-01 10:00:00"), 0)
        self.assertEqual(nfu.parse_time("garbage"), 0)


class Readings(unittest.TestCase):
    def test_a_desktop_without_a_battery_is_never_on_battery(self):
        r = calm()
        self.assertEqual(nfu.blockers(r), [])

    def test_the_text_of_each_blocker_names_the_fix(self):
        w = nfu.blockers(calm(on_battery=True, battery=40, snapshot=False))
        self.assertTrue(any("charger" in x for x in w) and any("snapper" in x for x in w))


if __name__ == "__main__":
    unittest.main(verbosity=2)

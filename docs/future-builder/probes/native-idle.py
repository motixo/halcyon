#!/usr/bin/env python3
"""Private nested compositor: idle sampling and stock panel orientation probes.
No installed settings, user bus or user display is used. No capture timer.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import selectors
import signal
import subprocess
import tempfile
import time

p = argparse.ArgumentParser()
p.add_argument("binary", type=Path)
p.add_argument("tools", type=Path, help="directory containing Xvfb and xdotool")
p.add_argument("output", type=Path)
p.add_argument("--idle-seconds", type=int, default=12, choices=range(1, 61))
p.add_argument("--analog-clock", action="store_true", help="exercise the new Clock only; skip stock panel probes")
p.add_argument("--component-pages", action="store_true", help="exercise Clock, Weather and Themes; skip stock panel probes")
a = p.parse_args()
root = a.output.resolve()
root.mkdir(parents=True, exist_ok=False)
runtime = Path(tempfile.mkdtemp(prefix="halcyon-fb-"))
runtime.chmod(0o700)
for name in ("config", "state", "cache", "data", "runtime", "desktop"):
    (root / name).mkdir(mode=0o700)
config = {"onboarded": True, "setup_dismissed": True, "enabled_widgets": ["clock", "weather"], "project_roots": []}
if a.analog_clock or a.component_pages:
    config.update(appearance={"family":"paper", "wallpaper_light":True}, clock={"face":"analog", "surface":"panel", "alignment":"center", "center_vertically":True})
(root / "config.json").write_text(json.dumps(config))
(root / "dconf-profile").write_text("")
before = (root / "config.json").read_bytes()
env = os.environ.copy()
for key in ("WAYLAND_DISPLAY", "WAYLAND_SOCKET", "COSMIC_SESSION_SOCK", "PANEL_NOTIFICATIONS_FD", "DAEMON_NOTIFICATIONS_FD", "DBUS_SESSION_BUS_ADDRESS", "SESSION_MANAGER", "DISPLAY", "DESKTOP_CARDS_SETTINGS_CAPTURE_DIR"):
    env.pop(key, None)
for kind in ("CONFIG", "STATE", "CACHE", "DATA"):
    env[f"XDG_{kind}_HOME"] = str(root / kind.lower())
env.update(XDG_RUNTIME_DIR=str(runtime), COSMIC_BACKEND="winit", WINIT_UNIX_BACKEND="x11", LIBGL_ALWAYS_SOFTWARE="1", RUST_LOG="warn", DCONF_PROFILE=str(root / "dconf-profile"), DESKTOP_CARDS_CONFIG=str(root / "config.json"), DESKTOP_CARDS_DESKTOP_ROOT=str(root / "desktop"))
env["LD_LIBRARY_PATH"] = str(a.tools.resolve().parent / "lib/x86_64-linux-gnu")
owned, logs = [], []
result = {"scope": "Private nested COSMIC; software compositor; app Vulkan adapter recorded separately; idle samples exclude diagnostic capture timer", "runs": [], "orientations": [], "binary_sha256": hashlib.sha256(a.binary.read_bytes()).hexdigest()}

def start(name, args, extra=None, pass_fds=()):
    log = (root / (name + ".log")).open("w")
    logs.append(log)
    proc = subprocess.Popen(args, env=env | (extra or {}), stdout=log, stderr=log, start_new_session=True, pass_fds=pass_fds)
    owned.append(proc)
    return proc

def stop(proc):
    try: os.killpg(proc.pid, signal.SIGTERM)
    except ProcessLookupError: pass
    try: proc.wait(timeout=3)
    except subprocess.TimeoutExpired:
        os.killpg(proc.pid, signal.SIGKILL)
        proc.wait(timeout=3)

def wait_for(fn, seconds=12):
    deadline = time.monotonic() + seconds
    while not fn():
        if time.monotonic() >= deadline: raise TimeoutError("Private fixture did not become ready")
        time.sleep(.05)

def stats(proc):
    assert proc.poll() is None
    fields = Path(f"/proc/{proc.pid}/stat").read_text().rsplit(")", 1)[1].split()
    status = dict(line.split(":", 1) for line in Path(f"/proc/{proc.pid}/status").read_text().splitlines() if ":" in line)
    gpu = []
    for fd in Path(f"/proc/{proc.pid}/fdinfo").iterdir():
        try:
            item = {k:v.strip() for k,v in (line.split(":",1) for line in fd.read_text().splitlines() if line.startswith("drm-"))}
            if item: gpu.append(item)
        except FileNotFoundError: pass
    switches = {}
    for thread in Path(f"/proc/{proc.pid}/task").iterdir():
        try:
            lines = thread.joinpath("status").read_text().splitlines()
            switches[thread.name] = int(next(line.split(":", 1)[1] for line in lines if line.startswith("voluntary_ctxt_switches:")))
        except FileNotFoundError: pass
    return {"cpu_seconds": (int(fields[11]) + int(fields[12])) / os.sysconf("SC_CLK_TCK"), "rss_kib": int(status["VmRSS"].split()[0]), "voluntary_switches": int(status["voluntary_ctxt_switches"]), "thread_voluntary_switches":switches, "drm": gpu}

try:
    read_fd, write_fd = os.pipe()
    start("xvfb", [str(a.tools / "Xvfb"), "-displayfd", str(write_fd), "-screen", "0", "1600x1000x24", "-nolisten", "tcp"], pass_fds=(write_fd,))
    os.close(write_fd)
    with selectors.DefaultSelector() as selector:
        selector.register(read_fd, selectors.EVENT_READ)
        assert selector.select(10), "Xvfb did not publish a display"
        env["DISPLAY"] = ":" + os.read(read_fd, 64).decode().strip()
    os.close(read_fd)
    bus = start("bus", ["dbus-daemon", "--session", "--nofork", "--print-address=1"])
    wait_for(lambda: (root / "bus.log").stat().st_size > 0)
    env["DBUS_SESSION_BUS_ADDRESS"] = (root / "bus.log").read_text().splitlines()[0]
    comp = start("compositor", ["/usr/bin/cosmic-comp", "--no-xwayland"])
    wait_for(lambda: any(x.is_socket() for x in runtime.glob("wayland-*")))
    assert comp.poll() is None
    env["WAYLAND_DISPLAY"] = str(next(x for x in runtime.glob("wayland-*") if x.is_socket()))
    result["outputs"] = subprocess.check_output(["cosmic-randr", "list", "--kdl"], env=env, text=True, timeout=10)
    pages = ("clock", "weather", "themes") if a.component_pages else (("clock",) if a.analog_clock else ("clock", "themes"))
    for page in pages:
        app = start("settings-" + page, [str(a.binary.resolve()), "--settings", page], {"LIBGL_ALWAYS_SOFTWARE":"0", "WAYLAND_DEBUG":"client"})
        time.sleep(4)
        subprocess.run(["import", "-display", env["DISPLAY"], "-window", "root", str(root / (page + ".png"))], env=env, check=True, timeout=10)
        first = stats(app)
        started = time.monotonic()
        time.sleep(a.idle_seconds)
        last = stats(app)
        elapsed = time.monotonic() - started
        common_threads = first["thread_voluntary_switches"].keys() & last["thread_voluntary_switches"].keys()
        switch_delta = sum(last["thread_voluntary_switches"][t] - first["thread_voluntary_switches"][t] for t in common_threads)
        result["runs"].append({"page": page, "seconds":elapsed, "cpu_percent_one_core":(last["cpu_seconds"]-first["cpu_seconds"])*100/elapsed, "surviving_threads_voluntary_switches_per_second":switch_delta/elapsed, "switches_note":"Lower bound for threads present at both samples; excludes exited/new threads and is not a timer-wakeup count", "first":first, "last":last})
        stop(app)
    # Stock panel only. Inspect actual Wayland anchors and configure sizes.
    def seed(ns, key, value):
        path = root / "config/cosmic" / ns / "v1" / key
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(value)
    if not (a.analog_clock or a.component_pages):
        seed("com.system76.CosmicPanel", "entries", '["Panel"]')
        for key, value in {"name":'"Panel"', "output":"All", "size":"M", "expand_to_edges":"true", "plugins_center":'Some(["com.system76.CosmicAppletTime"])'}.items():
            seed("com.system76.CosmicPanel.Panel", key, value)
    for anchor, bits in (() if a.analog_clock or a.component_pages else (("Left",7), ("Right",11))):
        seed("com.system76.CosmicPanel.Panel", "anchor", anchor)
        panel = start("panel-" + anchor.lower(), ["/usr/bin/cosmic-panel"], {"WAYLAND_DEBUG":"client"})
        time.sleep(2)
        assert panel.poll() is None, "Stock panel exited"
        raw = (root / ("panel-" + anchor.lower() + ".log")).read_text()
        events = [line.strip() for line in raw.splitlines() if "zwlr_layer_surface" in line and any(s in line for s in ("set_anchor", "set_size", ".configure("))]
        matched = any(re.search(r"set_anchor\("+str(bits)+r"\)", line) for line in events)
        assert matched, (anchor, events[-10:])
        result["orientations"].append({"anchor":anchor,"expected_anchor_bits":bits,"confirmed":matched,"protocol_events":events})
        stop(panel)
    result["configuration_unchanged"] = (root / "config.json").read_bytes() == before
    assert result["configuration_unchanged"]
finally:
    for proc in reversed(owned): stop(proc)
    for log in logs: log.close()
    result["owned_children_reaped"] = all(proc.returncode is not None for proc in owned)
    (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({k:v for k,v in result.items() if k not in ("runs", "orientations", "outputs")}, indent=2))

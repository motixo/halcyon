#!/usr/bin/env python3
"""Run the opt-in native-tree probe with private preferences and a private bus."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import time

p = argparse.ArgumentParser()
p.add_argument("binary", type=Path)
p.add_argument("output", type=Path)
p.add_argument("--test-name", default="measure_current_native_tree", choices=("measure_current_native_tree", "measure_clock_foundation", "measure_component_foundation", "measure_curated_presets"))
a = p.parse_args()
binary, root = a.binary.resolve(), a.output.resolve()
root.mkdir(parents=True, exist_ok=False)
for name in ("config", "data", "cache", "state", "runtime", "desktop", "output"):
    (root / name).mkdir(mode=0o700)
(root / "config.json").write_text(json.dumps({"onboarded": True, "setup_dismissed": True, "enabled_widgets": ["clock", "weather"], "project_roots": []}))
(root / "dconf-profile").write_text("")
repo = Path(__file__).resolve().parents[3]
bundle = root / "desktop/pictures/Desktop Cards"
bundle.parent.mkdir(parents=True)
bundle.symlink_to(repo / "assets/wallpapers", target_is_directory=True)
for ns, key, value in (("com.system76.CosmicPanel", "entries", '["Panel","Dock"]'), ("com.system76.CosmicPanel.Panel", "anchor", "Top"), ("com.system76.CosmicPanel.Dock", "anchor", "Bottom")):
    target = root / "desktop/system/cosmic" / ns / "v1" / key
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(value)
env = os.environ.copy()
for key in ("WAYLAND_DISPLAY", "WAYLAND_SOCKET", "DISPLAY", "DBUS_SESSION_BUS_ADDRESS", "COSMIC_SESSION_SOCK", "LIBGL_ALWAYS_SOFTWARE"):
    env.pop(key, None)
for suffix, name in (("CONFIG", "config"), ("DATA", "data"), ("CACHE", "cache"), ("STATE", "state"), ("RUNTIME", "runtime")):
    env[f"XDG_{suffix}_HOME" if suffix != "RUNTIME" else "XDG_RUNTIME_DIR"] = str(root / name)
env.update(HALCYON_BUILDER_PROBE_ROOT=str(root), DESKTOP_CARDS_CONFIG=str(root / "config.json"), DESKTOP_CARDS_DESKTOP_ROOT=str(root / "desktop"), DCONF_PROFILE=str(root / "dconf-profile"))
started = time.monotonic()
drm = {}
with (root / "probe.log").open("w") as log:
    result = subprocess.Popen(["dbus-run-session", "--", str(binary), "--exact", "settings::future_builder_probe::" + a.test_name, "--ignored", "--nocapture", "--test-threads=1"], env=env, stdout=log, stderr=log)
    while result.poll() is None:
        if time.monotonic() - started > 180:
            result.terminate()
            raise TimeoutError("Probe exceeded its deadline")
        pending = [result.pid]
        while pending:
            pid = pending.pop()
            try:
                pending.extend(int(n) for n in Path(f"/proc/{pid}/task/{pid}/children").read_text().split())
                for fd in Path(f"/proc/{pid}/fdinfo").iterdir():
                    try:
                        fields = dict(line.split(":", 1) for line in fd.read_text().splitlines() if line.startswith("drm-"))
                        if fields:
                            key = f'{pid}:{fields.get("drm-client-id", "unknown")}'
                            drm[key] = {k: v.strip() for k, v in fields.items()}
                    except (FileNotFoundError, PermissionError):
                        pass
            except (FileNotFoundError, PermissionError):
                pass
        time.sleep(.02)
metadata = {"binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "kernel": platform.release(), "elapsed_seconds": time.monotonic() - started, "exit_code": result.returncode, "source_sha256": {str(f.relative_to(repo)): hashlib.sha256(f.read_bytes()).hexdigest() for f in (repo / "src/settings/future_builder_probe.rs", repo / "src/native_test.rs")}, "load_average": os.getloadavg(), "private_bus": True, "home_unchanged": True}
governor = Path("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor")
metadata["cpu_governor"] = governor.read_text().strip() if governor.exists() else None
metadata["owned_process_drm_observations"] = list(drm.values())
(root / "environment.json").write_text(json.dumps(metadata, indent=2) + "\n")
print(json.dumps(metadata, indent=2))
raise SystemExit(result.returncode)

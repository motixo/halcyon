#!/usr/bin/env python3
"""Collect retained probe results into the small, portable evidence record."""
import csv
import hashlib
import json
import math
from pathlib import Path
import statistics
import subprocess

repo = Path(__file__).resolve().parents[3]
raw = repo / ".task-future-builder-baseline-20261026"
out = repo / "docs/future-builder/evidence"
def sha(path): return hashlib.sha256(path.read_bytes()).hexdigest()
def read(path): return json.loads(path.read_text())
def p95(values): return sorted(values)[math.ceil(.95*len(values))-1]
tree = read(raw / "tree-r2/output/samples.json")
env = read(raw / "tree-r2/environment.json")
native = read(raw / "native-r5/result.json")
summary = []
for row in tree["rows"]:
    summary.append({k:row[k] for k in ("page","backend","width","height","samples","process_rss_kib")} | {"p50_draw_readback_ms":statistics.median(row["event_and_draw_readback_ms"]),"p95_draw_readback_ms":p95(row["event_and_draw_readback_ms"]),"p95_event_dispatch_ms":p95(row["event_dispatch_ms"])})
source_paths = ["src/config.rs", "src/layout_state.rs", "src/drawer.rs", "src/settings.rs", "src/settings/future_builder_probe.rs", "src/native_test.rs", "src/desktop/profiles/format.rs", "src/desktop/islands.rs", "src/desktop/panels.rs", "companion/patches/0001-dock-islands.patch", "docs/future-builder/probes/fonts.rs", "docs/future-builder/probes/native-idle.py", "docs/future-builder/probes/settings-baseline.py"]
pinned = Path('/home/workstation/.cargo/git/checkouts/libcosmic-41009aea1d72760b/dc11d6a')
sources = {p:sha(repo/p) for p in source_paths}
sources.update({"pinned-libcosmic/"+p:sha(pinned/p) for p in ("iced/core/src/font.rs", "iced/core/src/text.rs", "iced/graphics/src/text.rs", "iced/graphics/src/text/paragraph.rs")})
fonts = list(csv.DictReader((raw/"fonts.csv").open()))
assert len(fonts) == 36
assert native["configuration_unchanged"] and native["owned_children_reaped"]
assert all(r["confirmed"] for r in native["orientations"])
tests = read(raw/"focused-tests.json")
assert all(r["exit_code"] == 0 for r in tests)
result = {
 "date":"2026-10-26", "scope":"Foundation design/capability/baseline probes; no future engine or package qualification",
 "head":subprocess.check_output(["git","rev-parse","HEAD"],cwd=repo,text=True).strip(),
 "dirty_tree":True, "cpu":next(line.split(":",1)[1].strip() for line in Path('/proc/cpuinfo').read_text().splitlines() if line.startswith('model name')),
 "gpu":{"pci":"0000:07:00.0","vendor":"1002","device":"1638","driver":"amdgpu","headless_engine_activity_observed":True,"native_window_gpu_rendering_proven":False},
 "tree_environment":env, "tree_summary":summary, "tree_raw":tree,
 "test_build_source_sha256":read(raw/"test-build-source.json"),
 "post_measurement_source_change":"rustfmt-only relocation of the cfg(test) module declaration; test body and native driver unchanged",
 "native":native, "font_cases":fonts, "focused_tests":tests,
 "source_sha256":sources,
 "captures":{str(p.relative_to(repo)):sha(p) for p in list((raw/"tree-r2/output").glob('*.png'))+list((raw/"native-r5").glob('*.png'))},
 "unmeasured":["compositor input-to-present latency","sustained presentation frame pacing","long-run memory cycles","cold full-size asset cost","future provider deduplication implementation","vertical Islands","physical hotplug/mixed scale"],
 "raw_root":".task-future-builder-baseline-20261026",
 "notes":["Native compositor uses software rendering; the native app's amdgpu handle alone does not prove hardware rendering. Headless GPU activity was verified separately.","Idle thread context switches are observations, not equivalent to timer wakeups.","GPU allocations can overlap process RSS; do not sum them as distinct physical memory.","All original concept images remain unchanged; generated concepts are not native acceptance evidence."]
}
(out/"baseline-20261026.json").write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({"font_cases":len(fonts),"tests":len(tests),"tree_samples":sum(r['samples'] for r in summary),"captures":len(result['captures']),"evidence_bytes":(out/'baseline-20261026.json').stat().st_size}))

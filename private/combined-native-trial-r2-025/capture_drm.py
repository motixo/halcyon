#!/usr/bin/python3
"""Ordinary-user public sysfs capture only; no context, installation or admission."""
import argparse,hashlib,json,sys
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--helpers',type=Path,required=True)
a=p.parse_args();root=a.helpers.resolve()
EXPECTED={'runtime_identity.py': '962599de58e0be3c1cfbb92e4a39f9335e4869a51a088fd6e957c328434e8844', 'bundle_contract.py': '232e6e5f4b0c7a801e7f9093926b0ecfb16bba0cdb270f1cb498599b9deb1662', 'verification.py': '0a258bca4b4c76d60da608c093c5bb2ed005ee8efe958f102452a5ea38ac2582'}
for name,sha in EXPECTED.items():
    if hashlib.sha256((root/name).read_bytes()).hexdigest()!=sha:raise ValueError('capture_helper_identity')
sys.path.insert(0,str(root))
from runtime_identity import drm_device_snapshot
from verification import bounded

def observe():
    first=drm_device_snapshot();second=drm_device_snapshot()
    if first!=second:raise ValueError('device_changed_during_capture')
    return dict(format='halcyon-drm-device-observation',version=1,devices=first[0],
                bindings=first[1],agreeing_snapshots=2,helper_sha256=EXPECTED)
print(json.dumps(bounded(observe,2),sort_keys=True))

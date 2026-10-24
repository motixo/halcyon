"""Future native read-only probe. Own connection only; no session fd consumption.
Run under timeout 3s with a complete frozen helper directory from commit9916baa.
"""
import argparse,hashlib,json,sys,time
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--helpers',type=Path,required=True);a=p.parse_args()
expected={'panel_configuration.py':'6be28029f5d1e6725a21e91ded03d0c882ab3206469f7facc1f272695b49c97f',
          'bundle_contract.py':'232e6e5f4b0c7a801e7f9093926b0ecfb16bba0cdb270f1cb498599b9deb1662'}
for name,digest in expected.items():
 if hashlib.sha256((a.helpers/name).read_bytes()).hexdigest()!=digest:raise SystemExit('frozen helper mismatch')
sys.path.insert(0,str(a.helpers))
from panel_configuration import output_names
start=time.monotonic()
try:
 names=output_names()
 result={'output_names':names,'state':'observed-not-admission'};code=0
except (ValueError,OSError,KeyError) as e:
 # Only protocol ValueErrors are safe structured diagnostics. OSErrors may
 # contain socket paths; preserve type/errno instead of arbitrary exception text.
 result={'state':'refused','error_type':type(e).__name__,
         'reason':str(e) if isinstance(e,ValueError) else 'wayland_io_or_environment',
         'errno':getattr(e,'errno',None)};code=1
result.update(elapsed_seconds=time.monotonic()-start,helper_sha256=expected,qualified=False)
print(json.dumps(result,sort_keys=True));raise SystemExit(code)

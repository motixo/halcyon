"""One fresh production configuration snapshot; no inventory/export/graphics work."""
import sys,pathlib,json,hashlib
H=pathlib.Path('/var/lib/ps025');B=pathlib.Path('/usr/lib/cosmic-ext-halcyon-islands')
r=json.loads((H/'integration2.json').read_text());p=B/'panel_configuration.py'
assert hashlib.sha256(p.read_bytes()).hexdigest()==r['helper_hashes'][p.name]
sys.path.insert(0,str(B));from panel_configuration import Snapshot,policy
profile=json.loads((H/'structural-runtime-profile.json').read_text())
s=Snapshot(policy(profile))
try:
 out={'production_resolver':{'counts':s.counts,'outputs':s.outputs,'identity':s.identity,'records':s.records,'changed':s.changed()},'qualified':False}
 (H/'results/trial-metadata.json').write_text(json.dumps(out,indent=2)+'\n');print(json.dumps(out))
finally:s.close()

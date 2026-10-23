#!/usr/bin/python3
"""Read-only comparison of actual untouched observations and retained candidate."""
import json,pathlib,sys
new_root=pathlib.Path(sys.argv[1]);old_root=pathlib.Path(sys.argv[2])
new=json.loads((new_root/'untouched-clock/theme-results.json').read_text());old=json.loads((old_root/'candidate/theme-results.json').read_text())
fields=['label','expected_dark','observed_native_is_dark','observed_portal_dark','auto_switch','matched']
project=lambda r:[{k:o[k] for k in fields} for o in r['automatic']['observations']]
a,b=project(new),project(old)
effective_fields=['label','expected_dark','observed_portal_dark','auto_switch','matched']
effective=lambda values:[{k:o[k] for k in effective_fields} for o in values]
same=effective(a)==effective(b)
consistent=lambda values:all(o['observed_native_is_dark'] in (None,str(o['observed_portal_dark']).lower()) for o in values)
proof={'new_untouched':a,'retained_candidate':b,'same_observed_effective_pattern':same,'same_raw_override_representation':a==b,'native_keys_consistent_with_actual_portal':consistent(a) and consistent(b),'complete_sequences':len(a)==len(b)==5,'untouched_execution_complete':new.get('execution_complete',False),'untouched_errors':new['errors'],'old_candidate_run':39987753102,'old_candidate_artifact':12516872732,'old_candidate_zip_sha256':'61851430b7c466719b26c985dda09edbf52abfceebcef050736c9e9e4afa82e0','native_limitation_confirmed_for_pinned_guest':len(a)==5 and same and consistent(a) and consistent(b) and not new['errors'] and any(not o['matched'] for o in a)}
(new_root/'clock-baseline-comparison.json').write_text(json.dumps(proof,indent=2)+'\n');print(json.dumps(proof,indent=2))

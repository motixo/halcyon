"""Independent saved-byte/native-failure summary; frames require human review."""
import json,pathlib,sys
root=pathlib.Path(sys.argv[1]);cases={}
for context in ['stock-native-blur','public-frosted','public-native-off']:
    variants={}
    for variant in ['immediate','preceding']:
        folder=root/('native-blur-'+context+'-'+variant)
        file=folder/'isolation-results.json'
        if not file.exists():continue
        state=json.loads(file.read_text());proofs=[]
        for p in state['input_proofs']:
            data=(folder/(p['saved_file_label']+'.txt')).read_bytes()
            proofs.append({'token':p['token'],'frame':p['frame'],'raw_byte_exact':data==p['token'].encode()})
        variants[variant]={'complete':state['complete'],'setup_error':state.get('setup_error'),'errors':state.get('errors',[]),
            'inputs':proofs,'treatment':state.get('treatment'),'readbacks':state.get('native_readbacks',[]),
            'triggers':state['triggers'],'final_journal':state.get('final_journal'),'native_failure_observed':state['native_failure_observed'],
            'overview_frames_require_review':True,'preceding_unqualified':state.get('preceding_unqualified')}
    cases[context]=variants or {'not_run':True}
(root/'native-blur-r2-summary.json').write_text(json.dumps({'scope':'Three original-guest native controls, no hour delay, no baseline repeat','publication':'0.2.5 ON HOLD','cases':cases},indent=2)+'\n')
for co,row in cases.items():
    print(co,[(v,d.get('native_failure_observed'),len(d.get('inputs',[]))) for v,d in row.items() if isinstance(d,dict)])

invalid=[co for co,variants in cases.items() if "immediate" not in variants or any(not d.get("complete") or d.get("setup_error") or not d.get("inputs") or not all(p["raw_byte_exact"] for p in d["inputs"]) for d in variants.values() if isinstance(d,dict))]
if invalid:raise SystemExit("Unqualified setup/input comparisons retained: "+",".join(invalid))

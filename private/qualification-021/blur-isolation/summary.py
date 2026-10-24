"""Read-only native evidence classification; CI green is not a native pass."""
import hashlib,json,pathlib,re,sys
root=pathlib.Path(sys.argv[1]);cases={}
for context in ['stock-baseline','stock-native-blur','public-frosted','public-native-off']:
    folder=root/('native-blur-'+context);state=json.loads((folder/'isolation-results.json').read_text())
    raw_proofs=[]
    for proof in state['input_proofs']:
        data=(folder/(proof['saved_file_label']+'.txt')).read_bytes()
        matched=data==proof['token'].encode()
        raw_proofs.append({'token':proof['token'],'frame':proof['frame'],'exact_saved_file_bytes':matched})
        if not matched:raise RuntimeError('Independent raw save mismatch: '+context)
    cases[context]={'complete':state['complete'],'setup_error':state['setup_error'],
        'native_failure_observed':state['native_failure_observed'],'actual_input_proofs':raw_proofs,
        'triggers':state['triggers'],'files_applet_pixmap_panics':state['final_journal']['files_applet_pixmap_panics'],'workspaces_panics':state['final_journal'].get('workspaces_panics',[]),'overview_frames_require_independent_review':True,
        'config':state['treatment'],'runtime_checks_survived_pending_frame_review':
            state['complete'] and not state['native_failure_observed'] and not state['setup_error']
            and len(state['triggers'])==2 and all(t.get('after_canary','').startswith('real exact save') for t in state['triggers'])}
    if not state['complete'] or state['setup_error']:raise RuntimeError('Setup prevented comparison: '+context)
# Both controls begin with exactly the same native-exported builder before their assigned intervention.
comparability={}
for a,b in [('stock-baseline','stock-native-blur'),('public-frosted','public-native-off')]:
    left=(root/('native-blur-'+a)/'native-builder-before.txt').read_bytes()
    right=(root/('native-blur-'+b)/'native-builder-before.txt').read_bytes()
    comparability[a+' vs '+b]={'identical_native_builder_before_intervention':left==right,
        'left_sha256':hashlib.sha256(left).hexdigest(),'right_sha256':hashlib.sha256(right).hexdigest()}
    if left!=right:print('::warning::Native builder comparison has uncontrolled pre-intervention differences: '+a+' / '+b)
result={'scope':'Immediate offline native blur isolation; not RX590 attribution or release acceptance',
        'publication':'0.2.5 ON HOLD','cases':cases,'comparability':comparability}
(root/'native-blur-summary.json').write_text(json.dumps(result,indent=2)+'\n')
for co,row in cases.items():
    print(co,'native_failure=',row['native_failure_observed'],'exact_inputs=',len(row['actual_input_proofs']),'runtime_checks_survived_pending_frame_review=',row['runtime_checks_survived_pending_frame_review'])
    if row['native_failure_observed']:print('::warning::Observed native compositor failure in '+co+'; artifact/log review required')

"""Postmortem of the sole R1 allocation. Reads preserved evidence, no VM calls."""
import hashlib,json,pathlib,subprocess,sys
from configuration import change_flags,fields,validate_reexport
out=pathlib.Path(sys.argv[1]).resolve();root=out/'native-blur-isolation-r1-40044224132'
record={'run':40044224132,'executed_infra':'d3aeeb82bb1499a10dde8c04f911c1c6c83ea557','allocation_used':1,'allocation_cap':1,
        'publication':'0.2.5 ON HOLD','causal_comparison_complete':False,'rx590_cause':'unproven','cases':{}}
for co in ['stock-baseline','stock-native-blur']:
    folder=root/('native-blur-'+co);state=json.loads((folder/'isolation-results.json').read_text())
    proofs=[]
    for proof in state['input_proofs']:
        data=(folder/(proof['saved_file_label']+'.txt')).read_bytes()
        assert data==proof['token'].encode()
        proofs.append({'token':proof['token'],'frame':proof['frame'],'byte_exact':True})
    lines=(folder/'full-native-journal.txt').read_text().splitlines()
    panics=[l for l in lines if 'panicked at' in l]
    copy_hashes=json.loads((folder/'executed-source-sha256.json').read_text())
    for name,digest in copy_hashes.items():
        actual=hashlib.sha256((folder/'executed-source'/name).read_bytes()).hexdigest()
        expected=hashlib.sha256(subprocess.check_output(['git','show',record['executed_infra']+':private/qualification-021/'+name])).hexdigest()
        assert actual==digest==expected,name
    record['cases'][co]={'runtime_complete_flag':state['complete'],'runtime_native_failure_flag_not_authoritative':state['native_failure_observed'],
        'setup_error':state['setup_error'],'input_proofs':proofs,'normal_closes':state['normal_closes'],
        'all_native_panics':panics,'final_journal_runtime_summary':state['final_journal'],
        'executed_source_copies_match_commit':True,'executed_source_file_count':len(copy_hashes)}
stock=record['cases']['stock-baseline']
stock['reviewed_outcome']={'immediate_overview_visibly_opened':True,'first_overview_and_post_input_passed':True,
    'second_overview_not_visible_in_retained_frame':True,'second_trigger_qualified_pass':False,
    'separate_native_workspaces_panic':True,'workspaces_pid_before_after':[1619,3434],
    'workspaces_error':'Surface::configure: Surface does not support the adapter queue family; ERROR_SURFACE_LOST_KHR; capture protocol error1',
    'compositor_pid_starttime':[1505,3966],'compositor_identity_stayed_same':True}
assert 'panicked at' not in (root/'native-blur-stock-baseline/after-preserved-sequence-journal-before.txt').read_text()
assert 'panicked at' in (root/'native-blur-stock-baseline/after-preserved-sequence-journal-after.txt').read_text()
stock['reviewed_outcome']['workspaces_panic_first_appears_after_second_trigger_journal']=True
for stage in ['baseline','final']:
    identities=json.loads((root/'native-blur-stock-baseline'/(stage+'-identity.txt')).read_text())
    stock[stage+'_workspaces_identity']=[{k:p[k] for k in ['pid','exe','sha256']} for p in identities if p['exe']=='/usr/bin/cosmic-workspaces']
blur=root/'native-blur-stock-native-blur'
before=(blur/'native-builder-before.txt').read_text();after=(blur/'native-builder-after.txt').read_text();desired,_=change_flags(before,True)
files=json.loads((blur/'final-config-state.txt').read_text())['files'];prefix='.config/cosmic/com.system76.CosmicTheme.Dark'
tint=bytes.fromhex(files[prefix+'.Builder/v2/text_tint']['hex']).decode();bg=bytes.fromhex(files[prefix+'/v2/background']['hex']).decode()
record['cases']['stock-native-blur']['offline_guard_diagnosis']={'native_export_after':fields(after),
    'source_defined_projection_validation':validate_reexport(desired,after,tint,bg),
    'native_flags_applied':{f:bytes.fromhex(files[prefix+'/v2/'+f]['hex']).decode().strip() for f in ['frosted_windows','frosted_system_interface','frosted_applets']},
    'canary_or_overview_attempted':False,'native_success_not_claimed':True}
for co in ['public-frosted','public-native-off']:record['cases'][co]={'outcome':'NOT RUN: wrapper stopped on prior setup guard; no second allocation authorized'}
record['causal_questions']={'equivalent_native_blur_overview_crash':'UNANSWERED; configured but trigger not reached',
    'hour_delay_needed_for_R3_compositor_panic':'UNANSWERED; public immediate case not reached',
    'native_blur_off_avoids_R3_crash':'UNANSWERED; public native-off case not reached',
    'software_offscreen_shader_init_hypothesis':'Source-supported inference; not causally confirmed/refuted by this attempt'}
archive=out/'native-blur-isolation-r1-40044224132.zip';digest=hashlib.sha256(archive.read_bytes()).hexdigest()
api=json.loads((out/'artifact-metadata.json').read_text());artifact=next(a for a in api if a['id']==12540977061)
assert artifact['digest']=='sha256:'+digest
record['artifact']={k:artifact[k] for k in ['id','name','size_in_bytes','digest','expires_at']}
selected=['native-blur-stock-baseline/isolation-results.json','native-blur-stock-baseline/005-immediate-overview.png','native-blur-stock-baseline/013-after-preserved-sequence-overview.png','native-blur-stock-baseline/018-reopened-2-input.png','native-blur-stock-baseline/full-native-journal.txt','native-blur-stock-native-blur/isolation-results.json','native-blur-stock-native-blur/native-builder-before.txt','native-blur-stock-native-blur/native-builder-after.txt','native-blur-stock-native-blur/native-import.txt','native-blur-stock-native-blur/final-config-state.txt','native-blur-stock-native-blur/001-setup-or-input-failure.png']
record['selected_evidence_sha256']={p:hashlib.sha256((root/p).read_bytes()).hexdigest() for p in selected}
(out/'findings.json').write_text(json.dumps(record,indent=2)+'\n')
print(json.dumps({'artifact':record['artifact'],'cases':{co:{k:v for k,v in row.items() if k in ['setup_error','all_native_panics','offline_guard_diagnosis','reviewed_outcome','outcome']} for co,row in record['cases'].items()}},indent=2))

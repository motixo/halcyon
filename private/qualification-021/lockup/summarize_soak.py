#!/usr/bin/python3
"""Read retained evidence only; do not infer passes from sockets or job state."""
import collections,json,pathlib,re,sys,statistics
root=pathlib.Path(sys.argv[1]);state=json.loads((root/'lockup-results.json').read_text())
inputs=[x['input'] for x in state['coverage'] if 'input' in x];closes=[x['core_close'] for x in state['coverage'] if 'core_close' in x]
raw=[]
for p in inputs:
 label=re.sub(r'^\d+-','',p['screenshot']).removesuffix('-input')
 files=sorted(root.glob(label+'-saved-file-*.txt'),key=lambda p:int(p.stem.rsplit('-',1)[1]))
 saved_bytes=files[-1].read_bytes() if files else None
 data=saved_bytes.decode('utf-8',errors='replace') if saved_bytes is not None else None
 raw.append({'token':p['token'],'readback':data,'byte_exact':saved_bytes==p['token'].encode('utf-8'),'line_exact':data is not None and data.rstrip('\n')==p['token'],'frame':p['screenshot'],'ocr':p['token_ocr_exact'],'phase':p['phase'],'elapsed':p['elapsed'],'geometry_source':p['native_editor'].get('geometry_source')})
resources=[json.loads(x) for x in (root/'resource-samples.jsonl').read_text().splitlines()] if (root/'resource-samples.jsonl').exists() else []
groups=collections.defaultdict(list)
for sample in resources:
 byexe=collections.defaultdict(list)
 for p in sample['processes']:byexe[p['exe']].append(p)
 for exe,ps in byexe.items():
  groups[exe].append({'t':sample['monotonic'],'rss_kib':sum(int(p.get('VmRSS','0 kB').split()[0]) for p in ps),'fds':sum(p['fd_count'] for p in ps),'threads':sum(int(p.get('Threads',0)) for p in ps),'pids':[p['pid'] for p in ps],'starttimes':[p['starttime'] for p in ps],'cpu_ticks':sum(p['utime']+p['stime'] for p in ps)})
stats={}
for exe,series in groups.items():
 stats[exe]={'samples':len(series),'first':series[0],'last':series[-1],'rss_min_kib':min(x['rss_kib'] for x in series),'rss_max_kib':max(x['rss_kib'] for x in series),'fds_max':max(x['fds'] for x in series),'threads_max':max(x['threads'] for x in series),'pid_sets':sorted(set(tuple(x['pids']) for x in series))}
journal=(root/'full-native-journal.txt').read_text().splitlines() if (root/'full-native-journal.txt').exists() else []
comp_panics=[l for l in journal if 'cosmic-comp[' in l and 'panicked at' in l]
pixmap_panics=[journal[i] for i,l in enumerate(journal) if 'panicked at' in l and 'iced_tiny_skia' in l and any('Create pixel map' in x for x in journal[i+1:i+4])]
kernel_failures=[l for l in journal if re.search(r' kernel: .*?(segfault|Out of memory|oom-kill|Killed process|GPU HANG|GPU reset|ring.*timed out)',l,re.I)]
identities={}
for stage in ['baseline','preflight','core-final','final']:
 file=root/(stage+'-identity.txt')
 if file.exists():identities[stage]={p['exe']:{'pid':p['pid'],'sha256':p['sha256']} for p in json.loads(file.read_text()) if p['exe'] in ['/usr/bin/cosmic-comp','/usr/bin/cosmic-panel','/usr/bin/cosmic-edit','/usr/bin/cosmic-settings-daemon'] or p['exe'].endswith('/cosmic-panel-islands')}
public_identities={}
for stage in ['preflight','core-final']:
 file=root/('resource-'+stage+'-public-identity.txt')
 if file.exists():public_identities[stage]=json.loads(file.read_text())
optional_counts=collections.defaultdict(lambda:{'observed':0,'changed_or_closed':0})
for row in state['coverage']:
 if 'optional' not in row:continue
 for g in row['optional']['gestures']:
  item=optional_counts[g['action']];item['observed']+=1;item['changed_or_closed']+=bool(g.get('changed',g.get('geometry_changed',g.get('closed',False))))
core_proofs=[x for x in raw if x['phase'] in ['preflight','soak']]
gaps=[b['monotonic']-a['monotonic'] for a,b in zip(resources,resources[1:])]
shape_passed=(state.get('observed_elapsed',0)>=3600 and len(state['checkpoints'])==13 and len(core_proofs)==45 and len({x['token'] for x in core_proofs})==45 and len(closes)==15)
result={'core_expected_shape_passed':shape_passed,'cohort':state['cohort'],'preflight_passed':state.get('preflight_passed',False),'hour_complete':state['hour_complete'],'elapsed':state.get('observed_elapsed'),'core_checkpoints':len(state['checkpoints']),'input_checks':len(inputs),'unique_tokens':len(set(x['token'] for x in inputs)),'exact_line_readbacks':sum(x['line_exact'] for x in raw),'byte_exact_readbacks':sum(x['byte_exact'] for x in raw),'core_input_checks_including_preflight':len(core_proofs),'core_byte_exact':sum(x['byte_exact'] for x in core_proofs),'timed_and_final_input_checks':sum(x['phase']=='soak' for x in raw),'normal_closes':sum(x['closed'] for x in closes),'core_input_hour_passed':shape_passed and state['hour_complete'] and all(x['byte_exact'] for x in core_proofs) and all(x['closed'] for x in closes),'full_native_journal_present':(root/'full-native-journal.txt').exists(),'native_session_failure_observed':bool(comp_panics or pixmap_panics),'kernel_crash_oom_gpu_reset_lines':kernel_failures,'compositor_panics':comp_panics,'files_applet_pixmap_panics':pixmap_panics,'resume_input_responded':state.get('resume_input_responded'),'resume_error':state.get('resume_error'),'failures':state['failures'],'optional_failures':state['optional_failures'],'optional_raw_counts_not_blanket_pass':dict(optional_counts),'public_identities':public_identities,'identities':identities,'resource_sample_count':len(resources),'resource_median_gap_sec':statistics.median(gaps) if gaps else None,'resource_max_gap_sec':max(gaps) if gaps else None,'raw_input_proofs':raw,'resources':stats}
print(json.dumps(result,indent=2))

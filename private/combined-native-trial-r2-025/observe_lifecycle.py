"""Read-only owned-child/recovery evidence for the isolated trial, not admission."""
import pathlib,json,os,sys,time
D=pathlib.Path('/var/lib/ps025');O=D/'results';B=pathlib.Path('/usr/lib/cosmic-ext-halcyon-islands')
def save(name,value):(O/name).write_text(json.dumps(value,indent=2)+'\n')
def main():
 if os.getuid()!=1000:raise ValueError('disposable_user_required')
 sys.path.insert(0,str(B));from runtime_identity import host_process,_stat,_owned_by_host
 mode=sys.argv[1];errors=[];result={'qualified':False,'phase':mode,'time':time.time(),'errors':errors}
 if mode=='before':
  ready=json.loads((O/'readiness.json').read_text());expected=json.loads((D/'integration2.json').read_text())['packages']['islands']['elf_sha256']
  host=host_process(ready['pid'],expected);result['host']=host;result['children']={};counts={}
  for p in pathlib.Path('/proc').iterdir():
   if not p.name.isdigit():continue
   try:
    if p.stat().st_uid!=1000 or (p/'exe').resolve().name!='cosmic-applets':continue
    process=_stat(p)
    if _owned_by_host(process,host,pathlib.Path('/proc')):
     if len(result['children'])>=64:raise ValueError('child_count_bound')
     result['children'][p.name]={'exe':str((p/'exe').resolve()),**process};counts['cosmic-applets']=counts.get('cosmic-applets',0)+1
   except FileNotFoundError:continue
  result['actual_counts']=counts
  try:
   meta=json.loads((O/'trial-metadata.json').read_text());config=meta['production_resolver'];result['resolved_configuration']=config
   if 'error' in config:errors.append({'reason':'configuration_refused','details':config})
   elif counts!=config['counts']:errors.append({'reason':'configured_child_counts_mismatch','expected':config['counts'],'actual':counts})
  except (OSError,KeyError,ValueError) as e:errors.append({'reason':'configuration_evidence_unavailable','type':type(e).__name__})
  save('observation.owned-children-before.json',result)
 elif mode=='after':
  before=json.loads((O/'observation.owned-children-before.json').read_text());actors={str(before['host']['pid']):before['host'],**before['children']};result['old_actors']={}
  for pid,old in actors.items():
   try:
    current=_stat(pathlib.Path('/proc')/pid);same=current['start']==old['start'];result['old_actors'][pid]={'old_start':old['start'],'current_start':current['start'],'same_actor_alive':same}
    if same:errors.append({'reason':'owned_actor_survived','pid':pid})
   except FileNotFoundError:result['old_actors'][pid]={'same_actor_alive':False,'absent':True}
   except ValueError as e:
    result['old_actors'][pid]={'stat_error':str(e)};errors.append({'reason':'old_actor_not_proved_reaped','pid':pid,'stat_error':str(e)})
  events=[json.loads(x) for x in (O/'trial-entry.jsonl').read_text().splitlines()];starts=[x for x in events if x['event']=='trial-start'];stocks=[x for x in events if x['event']=='stock-no-ticket'];result['entry_events']=events
  if len(starts)!=1 or not stocks:errors.append({'reason':'single_trial_fresh_stock_event_missing'})
  else:
   initial=starts[0]['descriptors'].get('PANEL_NOTIFICATIONS_FD');fresh=stocks[-1]['descriptors'].get('PANEL_NOTIFICATIONS_FD');result['notification_descriptor_before']=initial;result['notification_descriptor_after']=fresh
   if not initial or not fresh or (initial['device'],initial['inode'])==(fresh['device'],fresh['inode']):errors.append({'reason':'notification_descriptor_not_fresh'})
   stocks_alive=[]
   for p in pathlib.Path('/proc').iterdir():
    if not p.name.isdigit():continue
    try:
     if p.stat().st_uid!=1000 or (p/'exe').resolve()!=pathlib.Path('/usr/bin/cosmic-panel'):continue
     raw=(p/'environ').read_bytes();
     if len(raw)>65536:raise ValueError('stock_environment_bound')
     fd=next((v.partition(b'=')[2] for v in raw.split(b'\0') if v.startswith(b'PANEL_NOTIFICATIONS_FD=')),None)
     target=os.readlink(p/'fd'/fd.decode()) if fd else None;stocks_alive.append({'pid':int(p.name),'stat':_stat(p),'notification_fd_link':target})
    except FileNotFoundError:continue
   result['stock_processes']=stocks_alive
   if len(stocks_alive)!=1:errors.append({'reason':'single_stock_host_missing','count':len(stocks_alive)})
   elif fresh and stocks_alive[0]['notification_fd_link']!='socket:['+str(fresh['inode'])+']':errors.append({'reason':'stock_notification_descriptor_binding_mismatch'})
  save('observation.owned-children-after.json',result)
 else:raise ValueError('mode')
 print(json.dumps({'qualified':False,'phase':mode,'errors':errors}));return int(bool(errors))
if __name__=='__main__':raise SystemExit(main())

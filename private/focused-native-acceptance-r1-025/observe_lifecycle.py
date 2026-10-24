"""Bounded read-only lifecycle diagnostic for the disposable trial, never admission."""
import hashlib,json,os,pathlib,sys,time

D=pathlib.Path('/var/lib/ps025'); B=pathlib.Path('/usr/lib/cosmic-ext-halcyon-islands')
MAX_PIDS=4096; MAX_ACTORS=128; MAX_DEPTH=64; MAX_STAT=4096
MAX_JSON=1024*1024; SECONDS=5
APPLET='/usr/bin/cosmic-applets'; STOCK='/usr/bin/cosmic-panel'
MAX_TASKS=512

def bounded(path,limit):
    with path.open('rb') as f: value=f.read(limit+1)
    if len(value)>limit: raise ValueError('record_byte_bound')
    return value

def stat_record(path):
    raw=bounded(path/'stat',MAX_STAT); prefix,sep,tail=raw.rpartition(b') ')
    fields=tail.split()
    if not sep or len(fields)<20 or prefix.split(b' ',1)[0]!=path.name.encode():
        raise ValueError('process_stat_invalid')
    threads=int(fields[17])
    if threads<1: raise ValueError('thread_count_invalid')
    return dict(pid=int(path.name),state=fields[0].decode(),start=str(int(fields[19])),threads=threads,
                parent=int(fields[1]),group=int(fields[2]),sid=int(fields[3]))

class Capture:
    def __init__(self,mode,root=D,proc=pathlib.Path('/proc'),uid=1000):
        if mode not in ('before','after'): raise ValueError('mode')
        self.root=root; self.proc=proc; self.uid=uid; self.deadline=time.monotonic()+SECONDS
        self.path=root/'results'/('observation.owned-children-'+mode+'.json')
        self.r={'format':'halcyon-trial-lifecycle','version':2,'phase':mode,'qualified':False,
                'time':time.time(),'complete':False,'actors':{},'errors':[],
                'bounds':{'pids':MAX_PIDS,'actors':MAX_ACTORS,'ancestry':MAX_DEPTH,
                          'stat_bytes':MAX_STAT,'output_bytes':MAX_JSON,'seconds':SECONDS}}
        self.r['bounds']['tasks_per_actor']=MAX_TASKS
        # Never replace an earlier observation/receipt, even on a repeated invocation.
        with self.path.open('x') as f: json.dump(self.r,f)
        self.save()
    def check(self):
        if time.monotonic()>self.deadline: raise TimeoutError('diagnostic_deadline')
    def save(self):
        raw=(json.dumps(self.r,indent=2)+'\n').encode()
        if len(raw)>MAX_JSON: raise ValueError('observation_byte_bound')
        tmp=self.path.with_suffix('.writing')
        with tmp.open('xb') as f: f.write(raw); f.flush(); os.fsync(f.fileno())
        os.replace(tmp,self.path)
    def error(self,reason,pid=None,exc=None,**details):
        item={'reason':reason,**details}
        if pid is not None: item['pid']=str(pid)
        if exc is not None: item.update(type=type(exc).__name__,errno=getattr(exc,'errno',None))
        self.r['errors'].append(item); self.save()
    def readjson(self,path): return json.loads(bounded(path,MAX_JSON))
    def activity(self,path,record):
        """A dead leader can have live workers; a dead single-thread zombie cannot."""
        dead=record['state'] in ('Z','X')
        result={'leader_state':record['state'],'threads_reported':record['threads'],
                'dead_single_thread':dead and record['threads']==1,'live':not dead}
        if result['dead_single_thread']:
            result['live_workers']=0; result['reaped']=False
            return result
        if not dead:
            result['live_workers']=None  # num_threads is not an invented live-worker count.
            return result
        # Z/X with multiple tasks needs actual task states, not just leader state.
        tasks={}
        for task in (path/'task').iterdir():
            self.check()
            if not task.name.isdigit():continue
            if len(tasks)>=MAX_TASKS:raise ValueError('task_count_bound')
            try:tasks[task.name]=stat_record(task)
            except FileNotFoundError:continue
        after=stat_record(path)
        if any(after[k]!=record[k] for k in ('start','group','sid','threads','state')):
            raise ValueError('task_snapshot_changed')
        if len(tasks)!=record['threads']:raise ValueError('task_snapshot_incomplete')
        live=[pid for pid,task in tasks.items() if task['state'] not in ('Z','X')]
        result.update(tasks=tasks,live_worker_tids=live,live_workers=len(live),live=bool(live))
        if not live:result['dead_multithread_pending']=True
        return result
    def scan(self):
        count=0
        for p in self.proc.iterdir():
            self.check()
            if not p.name.isdigit(): continue
            count+=1
            if count>MAX_PIDS: raise ValueError('process_count_bound')
            try:
                if p.stat().st_uid!=self.uid: continue
                record=stat_record(p)
            except FileNotFoundError: continue
            except (OSError,ValueError) as e:
                # Without stat, unrelatedness cannot be proved. No exe access.
                self.error('same_uid_stat_unreadable_ownership_unknown',p.name,e); continue
            yield p,record
    def descends(self,record,parent_pid,group=None,sid=None,start=None):
        seen=set()
        for _ in range(MAX_DEPTH):
            self.check()
            if group is not None and (record['group']!=group or record['sid']!=sid or int(record['start'])<int(start)):
                return False
            parent=record['parent']
            if parent==parent_pid: return True
            if parent<=1 or parent in seen: return False
            seen.add(parent); record=stat_record(self.proc/str(parent))
        raise ValueError('ancestry_depth_bound')
    def remember(self,key,record):
        if key not in self.r['actors'] and len(self.r['actors'])>=MAX_ACTORS:
            raise ValueError('actor_count_bound')
        self.r['actors'][key]={'stat':record,'verified':False}; self.save()
        return self.r['actors'][key]
    def before(self,host):
        self.r.update(host=host,children={},actual_counts={}); self.save()
        for p,st in self.scan():
            # Stat is inspected before exe; unrelated inaccessible executables are irrelevant.
            if st['pid']==host['pid'] or st['group']!=host['pid']: continue
            actor=self.remember(p.name,st)
            try:
                if st['state'] in ('Z','X'): raise ValueError('group_member_not_live')
                if not self.descends(st,host['pid'],host['pid'],host['sid'],host['start']):
                    raise ValueError('group_member_ancestry_unproved')
                actor['owned']=True; self.save()
                exe=str((p/'exe').resolve(strict=True)); actor['exe']=exe; self.save()
                if stat_record(p)!=st: raise ValueError('owned_actor_changed')
                if exe!=APPLET: raise ValueError('unexpected_owned_executable')
                actor['verified']=True
                self.r['children'][p.name]={**st,'exe':exe}
                self.r['actual_counts']['cosmic-applets']=len(self.r['children'])
                self.save()
            except (OSError,ValueError) as e:
                actor['error']={'type':type(e).__name__,'errno':getattr(e,'errno',None),'reason':str(e)[:240]}
                self.error('owned_or_group_actor_unverified',p.name,e)
        try:
            config=self.readjson(self.root/'results/trial-metadata.json')['production_resolver']
            self.r['resolved_configuration']=config; self.save()
            if 'error' in config: self.error('configuration_refused')
            elif self.r['actual_counts']!=config['counts']:
                self.error('configured_child_counts_mismatch',expected=config['counts'],actual=self.r['actual_counts'])
        except (OSError,ValueError,KeyError) as e: self.error('configuration_evidence_unavailable',exc=e)
    def after(self):
        before=None
        try:
            before=self.readjson(self.root/'results/observation.owned-children-before.json')
            if before.get('format')!='halcyon-trial-lifecycle' or before.get('version')!=2 or before.get('phase')!='before':
                raise ValueError('before_format')
            if not before.get('complete') or before.get('errors'): self.error('before_observation_incomplete')
            host=before['host']; actors={str(host['pid']):host}
            if len(before.get('actors',{}))>MAX_ACTORS: raise ValueError('before_actor_count_bound')
            for pid,item in before.get('actors',{}).items(): actors[pid]=item['stat']
            for pid,old in actors.items():
                if not pid.isdecimal() or int(pid)<=0 or not str(old['start']).isdecimal():
                    raise ValueError('before_actor_identity_invalid')
                if old.get('pid',int(pid))!=int(pid): raise ValueError('before_actor_pid_mismatch')
            self.r['old_actors']={}; self.save()
            for pid,old in actors.items():
                self.check(); item={'before':old}; self.r['old_actors'][pid]=item; self.save()
                try:
                    current=stat_record(self.proc/pid); item['after']=current
                    same=current['start']==old['start']; item['same_identity']=same
                    item['same_pgid']=current['group']==old.get('group',host['pid'])
                    if same:
                        # Single-thread dead zombies are recorded, not live-actor failures.
                        item['activity']=self.activity(self.proc/pid,current)
                        if item['activity']['live']:self.error('owned_actor_survived',pid)
                        elif item['activity'].get('dead_multithread_pending'):
                            self.error('owned_task_completion_unproved',pid)
                    else: item['pid_reused_old_identity_absent']=True
                except FileNotFoundError: item['absent']=True
                except (OSError,ValueError) as e:
                    item['error']={'type':type(e).__name__,'errno':getattr(e,'errno',None)}
                    self.error('old_actor_state_unreadable',pid,e)
                finally: self.save()
            # Also reject new/previously unrecorded members of the original group.
            for p,st in self.scan():
                if st['group']==host['pid'] and int(st['start'])>=int(host['start']):
                    if p.name not in actors or st['start']!=actors[p.name]['start']:
                        actor=self.remember(p.name,st)
                        try:
                            actor['activity']=self.activity(p,st);self.save()
                            if actor['activity']['live'] or actor['activity'].get('dead_multithread_pending'):
                                self.error('unaccounted_old_group_member',p.name)
                        except FileNotFoundError:actor['absent_after_stat']=True;self.save()
                        except (OSError,ValueError) as e:self.error('old_group_task_state_unreadable',p.name,e)
        except Exception as e:
            self.error('before_unavailable_or_invalid',exc=e)
        finally:
            # A malformed/partial before record never skips stock observation.
            self.stock()
    def stock(self):
        try:
            events=[json.loads(x) for x in bounded(self.root/'results/trial-entry.jsonl',65536).splitlines()]
            starts=[x for x in events if x.get('event')=='trial-start']
            stocks=[x for x in events if x.get('event')=='stock-no-ticket']
            if len(starts)!=1 or len(stocks)!=1: raise ValueError('single_trial_stock_event_required')
            first,last=starts[0],stocks[0]; self.r['entry_events']=events
            old=first['descriptors']['PANEL_NOTIFICATIONS_FD']; fresh=last['descriptors']['PANEL_NOTIFICATIONS_FD']
            self.r.update(notification_descriptor_before=old,notification_descriptor_after=fresh,stock_processes=[]); self.save()
            if last['monotonic']<=first['monotonic']: self.error('stock_event_not_after_trial')
            if (old['device'],old['inode'])==(fresh['device'],fresh['inode']): self.error('notification_descriptor_not_fresh')
            for p,st in self.scan():
                # stock_host starts a new session/group; only its direct leader can qualify.
                if st['parent']!=last['pid'] or st['group']!=st['pid'] or st['sid']!=st['pid']: continue
                actor=self.remember(p.name,st)
                try:
                    if st['state'] in ('Z','X'): raise ValueError('stock_not_live')
                    exe=str((p/'exe').resolve(strict=True)); actor['exe']=exe; self.save()
                    if exe!=STOCK: raise ValueError('stock_executable_mismatch')
                    raw=bounded(p/'environ',65536)
                    values=[v.partition(b'=')[2] for v in raw.split(b'\0') if v.startswith(b'PANEL_NOTIFICATIONS_FD=')]
                    if len(values)!=1 or not values[0].isdigit(): raise ValueError('stock_notification_fd_missing')
                    link=os.readlink(p/'fd'/values[0].decode()); actor['notification_fd_link']=link
                    if stat_record(p)!=st: raise ValueError('stock_identity_changed')
                    if link!=f"socket:[{fresh['inode']}]": raise ValueError('stock_notification_descriptor_binding_mismatch')
                    actor['verified']=True; self.r['stock_processes'].append({'exe':exe,**st,'notification_fd_link':link}); self.save()
                except (OSError,ValueError) as e:
                    actor['error']={'type':type(e).__name__,'reason':str(e)[:240]}; self.error('stock_actor_unverified',p.name,e)
            if len(self.r['stock_processes'])!=1: self.error('single_bound_stock_host_missing',count=len(self.r['stock_processes']))
        except (OSError,ValueError,KeyError,TypeError) as e: self.error('stock_evidence_unavailable',exc=e)
    def finish(self):
        self.r['complete']=not self.r['errors']; self.r['elapsed_seconds']=SECONDS-(self.deadline-time.monotonic()); self.save()
        return int(bool(self.r['errors']))

def main():
    if os.getuid()!=1000: raise ValueError('disposable_user_required')
    mode=sys.argv[1]; c=Capture(mode)
    try:
        if mode=='before':
            receipt=c.readjson(D/'integration2.json'); ready=c.readjson(D/'results/readiness.json')
            # Frozen installed helper, rebound to the new reviewed package receipt by the parent.
            raw=bounded(B/'runtime_identity.py',1024*1024)
            if hashlib.sha256(raw).hexdigest()!=receipt['helper_hashes']['runtime_identity.py']:
                raise ValueError('runtime_helper_identity_mismatch')
            sys.path.insert(0,str(B)); from runtime_identity import host_process
            host=host_process(ready['pid'],receipt['packages']['islands']['elf_sha256'])
            if host['start']!=ready['start_ticks']: raise ValueError('readiness_host_changed')
            c.before(host)
        else: c.after()
    except Exception as e: c.error('capture_incomplete',exc=e,detail=str(e)[:240])
    rc=c.finish(); print(json.dumps({'qualified':False,'phase':mode,'complete':c.r['complete'],'errors':c.r['errors']})); return rc

if __name__=='__main__': raise SystemExit(main())

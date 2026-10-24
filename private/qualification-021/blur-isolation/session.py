#!/usr/bin/python3
"""Immediate native overview/blur comparison. No product patch or synthetic save."""
import base64, difflib, hashlib, json, pathlib, re, shlex, sys, time, traceback
SRC = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(SRC.parent / 'theme'))
sys.path.insert(0, str(SRC.parent / 'lockup'))
import theme_session as h  # Definitions-only import; exercise.py is never imported.
from calibrated_controls import Pixels, validate_samples
from configuration import FLAGS, fields, change_flags, verify_media, validate_reexport, export_values, ron_tokens
c = h.c
work, ev = c.work, c.ev
context = sys.argv[4]
if context not in ['stock-baseline','stock-native-blur','public-frosted','public-native-off']:
    raise ValueError('Unknown isolation context')
variant=sys.argv[5] if len(sys.argv)>5 else 'immediate'
recover=len(sys.argv)>6 and sys.argv[6]=='recover'
if variant not in ['immediate','preceding']:raise ValueError('Unknown stimulus variant')
public = context.startswith('public-')
class ProofError(RuntimeError):pass
class InputError(RuntimeError):pass
class HarnessIssue(RuntimeError):pass
result = {'variant':variant,'errors':[],'context': context, 'public': public, 'started_realtime': time.time(),
          'input_proofs': [], 'triggers': [], 'normal_closes': [], 'setup_error': None,
          'native_failure_observed': False, 'complete': False, 'injected_mode_events': False}
if recover:
    result=json.loads((ev/'isolation-results.json').read_text());result['setup_error']=None;result['complete']=False
    h.count=result.get('count',0)
CAL = json.loads((SRC.parent/'theme/followup/control-calibration.json').read_text())
EDITOR = '61f32f20df9c4da800b0df8054cb0b7ec6c65ccd4b9d11679f7f6c7303dc1149'
MAIN = '0abb0f393b14ba0a5ddf791a719c391dfa9a8a04e39196a1caddc8ff7028a01f'
COMP = '2399b768cc6a4599a6495d0865cf4dc6807cfee2c6e46b3d7466a800f903be5e'

def save():
    result['count']=h.count
    (ev/'isolation-results.json').write_text(json.dumps(result, indent=2)+'\n')

def command(label, cmd, user=False, required=True, timeout=60):
    return h.command(label, cmd, user, required, timeout)

def screen(label):
    name, path, words = h.screen(label)
    result.setdefault('frames', []).append(name)
    save()
    return name, path, words

def observe(label):
    raw = command(label, 'systemd-run --user --quiet --collect --wait --pipe /usr/bin/python3 /var/tmp/lockup-probe/wayland_observer.py', True)
    return json.loads(next(line for line in raw.splitlines() if line.startswith('{')))

def editor_window(label):
    hits=[]
    try:
        observed=observe(label)
        hits=[w for w in observed['windows'] if not w.get('closed') and 'cosmicedit' in w.get('app_id','').lower().replace('-','')]
    except Exception as e:
        result.setdefault('optional_observer_errors',[]).append({'label':label,'error':str(e)})
    if len(hits)>1:raise ProofError('Ambiguous multiple native editor windows')
    if len(hits)==1 and 'geometry' in hits[0]:
        hits[0]['geometry_source']='native protocol';return hits[0]
    # Actual executable/filename plus fresh calibrated chrome, never wallpaper.
    rows=identities(label+'-process')
    editors=[r for r in rows if r['exe']=='/usr/bin/cosmic-edit' and r['sha256']==EDITOR and '/home/probe/lockup-input.txt' in r['args']]
    if len(editors)!=1:raise ProofError('Actual pinned editor identity missing/ambiguous')
    name,path,_=screen(label+'-pixel-focus')
    from focus_pixels import locate
    try:g=locate(path)
    except ValueError as e:raise HarnessIssue(str(e))
    result.setdefault('optional_observer_errors',[]).append({'label':label,'fallback_frame':name,'native_geometry_unqualified':True})
    return {'app_id':'com.system76.CosmicEdit','geometry':g,'geometry_source':'fresh screenshot focus only','process':editors[0]}

def identities(label):
    raw = command(label, """python3 - <<'IDENTITY'
import os,pathlib,hashlib,json
rows=[]
for p in pathlib.Path('/proc').iterdir():
 if not p.name.isdigit():continue
 try:
  exe=os.readlink(p/'exe')
  if exe in ['/usr/bin/cosmic-comp','/usr/bin/cosmic-edit','/usr/bin/cosmic-panel','/usr/bin/cosmic-workspaces','/usr/bin/halcyon','/usr/bin/cosmic-settings-daemon','/usr/libexec/xdg-desktop-portal-cosmic']:
   stat=(p/'stat').read_text().rsplit(')',1)[1].split()
   rows.append({'pid':int(p.name),'starttime':int(stat[19]),'exe':exe,'sha256':hashlib.sha256((p/'exe').read_bytes()).hexdigest(),'args':(p/'cmdline').read_bytes().decode().replace('\\0',' ')})
 except OSError:pass
print(json.dumps(rows))
IDENTITY""")
    return json.loads(raw)

def compositor(rows):
    found=[r for r in rows if r['exe']=='/usr/bin/cosmic-comp']
    if len(found)!=1 or found[0]['sha256']!=COMP:
        raise ProofError('Pinned compositor missing or wrong ELF')
    return found[0]

def journal(label):
    raw=command(label,'journalctl -b -o short-monotonic --no-pager',required=False)
    lines=raw.splitlines()
    panics=[x for x in lines if 'cosmic-comp[' in x and 'panicked at' in x]
    applets=[lines[i] for i,x in enumerate(lines) if 'panicked at' in x and 'iced_tiny_skia' in x and any('Create pixel map' in y for y in lines[i+1:i+4])]
    workspace=[x for x in lines if 'panicked at' in x and ('cosmic-workspaces-' in x or 'cosmic-workspaces[' in x)]
    return {'compositor_panics':panics,'workspaces_panics':workspace,'files_applet_pixmap_panics':applets}

def export(label):
    command(label+'-command','/usr/bin/cosmic-settings appearance export /var/tmp/blur-export.ron',True)
    raw=command(label,'cat /var/tmp/blur-export.ron')
    try:fields(raw)
    except ValueError as e:raise ProofError(str(e))
    return raw

def native_readback(label, requested):
    after=export(label+'-export');snapshot=h.snapshot(label+'-state');raw=snapshot['files']
    if public:
        try:config=json.loads(bytes.fromhex(raw['.config/halcyon/config.json']['hex']).decode())
        except (KeyError,ValueError) as e:raise ProofError('Public material configuration unavailable: '+str(e))
        if (config.get('appearance',{}).get('family'),config.get('theme'))!=('frosted','ocean'):raise ProofError('Public material/profile changed during native control')
    effective=fields(after);wanted='true' if context in ['stock-native-blur','public-frosted'] else 'false'
    if any(effective[f]!=wanted for f in FLAGS):raise ProofError('Effective native frosting flags differ from assigned control')
    prefix='.config/cosmic/com.system76.CosmicTheme.Dark'
    for f in FLAGS:
        path=prefix+'/v2/'+f
        if path not in raw or bytes.fromhex(raw[path]['hex']).decode().strip()!=wanted:
            raise ProofError('Actual generated native theme flag missing/mismatched: '+f)
    values=export_values(requested)
    stored={k[len(prefix+'.Builder/v2/'):]:bytes.fromhex(v['hex']).decode() for k,v in raw.items() if k.startswith(prefix+'.Builder/v2/')}
    mismatches=[key for key,value in values.items() if key not in stored or ron_tokens(stored[key])!=ron_tokens(value)]
    controls=set(FLAGS)|{'frosted','frosted_panel','alpha_map','frosted_maximized_apps'}
    if result.get('native_import_applied') and controls.intersection(mismatches):
        raise ProofError('Actual stored blur controls differ from requested input: '+','.join(sorted(controls.intersection(mismatches))))
    for key in ['frosted','frosted_panel','alpha_map','frosted_maximized_apps']:
        path=prefix+'/v2/'+key
        if path in raw and ron_tokens(bytes.fromhex(raw[path]['hex']).decode())!=ron_tokens(values[key]):
            raise ProofError('Generated native blur parameter differs: '+key)
    projection=None
    if result.get('native_import_applied'):
        try:projection=validate_reexport(requested,after,stored.get('text_tint',''),bytes.fromhex(raw[prefix+'/v2/background']['hex']).decode())
        except (ValueError,KeyError) as e:projection={'unexpected_noncontrol_export_difference':str(e)}
    diff=''.join(difflib.unified_diff(requested.splitlines(True),after.splitlines(True),fromfile='requested',tofile='native-export'))
    (ev/(label+'-export.diff')).write_text(diff)
    proof={'effective':effective,'stored_fields_matched':len(values)-len(mismatches),'stored_field_differences':mismatches,
           'native_export_projection':projection,'export_diff_retained':bool(diff),'relevant_controls_verified':True}
    result.setdefault('native_readbacks',[]).append({'label':label,**proof});save()
    return after,proof

def treatment(rechecking=False):
    if not rechecking:
        h.snapshot('treatment-before');before=export('native-builder-before')
        original=fields(before);expected='true' if public else 'false'
        if any(original[f]!=expected for f in FLAGS):raise ProofError('Initial global effects differ from assigned public/stock treatment')
        desired=before;change=None
        if context in ['stock-native-blur','public-native-off']:
            desired,change=change_flags(before,context=='stock-native-blur')
        (ev/'native-requested.ron').write_text(desired)
        result['treatment_plan']={'before':original,'change':change};save()
        if change is not None:
            encoded=base64.b64encode(desired.encode()).decode()
            command('stage-native-import','umask 077; echo '+shlex.quote(encoded)+' | base64 -d > /var/tmp/blur-treatment.ron',True)
            command('native-import','/usr/bin/cosmic-settings appearance import /var/tmp/blur-treatment.ron',True)
            result['native_import_applied']=True;save();time.sleep(6)
        result['configuration_applied']=True;save()
    desired=(ev/'native-requested.ron').read_text()
    after,proof=native_readback('treatment-after'+('-recovery' if rechecking else ''),desired)
    result['treatment']={'exported_before':result['treatment_plan']['before'],'exported_after':fields(after),
        'change':result['treatment_plan']['change'],'readback':proof,
        'requested_builder_sha256':hashlib.sha256(desired.encode()).hexdigest(),
        'after_builder_sha256':hashlib.sha256(after.encode()).hexdigest(),
        'widget_blur_requests_retained':public,'scope':'active Dark global windows/applets/system-interface effects; panel false'}
    result.setdefault('treatment_applied_realtime',time.time());save();screen('native-treatment-visible')

def install_helpers():
    for rel in ['wayland_observer.py','resources.py','protocols/cosmic-toplevel-info-unstable-v1.xml','protocols/ext-foreign-toplevel-list-v1.xml']:
        raw=(SRC.parent/'lockup'/rel).read_bytes();target='/var/tmp/lockup-probe/'+rel
        command('helper-'+pathlib.Path(rel).stem,'mkdir -p '+shlex.quote(str(pathlib.PurePosixPath(target).parent))+' && echo '+shlex.quote(base64.b64encode(raw).decode())+' | base64 -d > '+shlex.quote(target))

def launch_editor(seq):
    unit='blur-editor-'+str(seq)
    command('launch-'+unit,'systemd-run --user --collect --property=Type=forking --property=GuessMainPID=yes --unit='+unit+' /usr/bin/cosmic-edit /home/probe/lockup-input.txt',True)
    time.sleep(5)
    diagnostic=command('unit-'+unit,'systemctl --user show '+unit+'.service -p Type -p MainPID -p Result -p ActiveState -p SubState; journalctl --user -b -u '+unit+'.service --no-pager',True)
    if seq==0 and not ('Type=forking' in diagnostic and 'ActiveState=active' in diagnostic and 'Result=success' in diagnostic):
        raise HarnessIssue('Initial native forking service is not active/successful')
    rows=identities('editor-identity-'+str(seq))
    if not any(r['exe']=='/usr/bin/cosmic-edit' and r['sha256']==EDITOR for r in rows):
        raise ProofError('Actual native editor ELF missing')
    result['editor_launched']=True;save()
    return editor_window('editor-open-'+str(seq))

def canary(label):
    w=editor_window(label+'-before');g=w['geometry']
    c.click(max(40,min(1240,g['x']+g['width']//2)),max(100,min(670,g['y']+g['height']//2)))
    result['input_serial']=result.get('input_serial',0)+1;save()
    token='b'+context.replace('-','')+str(result['input_serial']).zfill(3)
    before,_,_=screen(label+'-before-input')
    c.key('ctrl-a');c.text(token);c.key('ctrl-s');time.sleep(1)
    deadline=time.monotonic()+5;attempt=0
    while True:
        raw=command(label+'-saved-file-'+str(attempt),'cat /home/probe/lockup-input.txt')
        if raw==token or time.monotonic()>=deadline:break
        time.sleep(.5);attempt+=1
    name,_,_=screen(label+'-input')
    after={'metadata_optional':True}
    try:after=observe(label+'-after')
    except Exception as e:after={'observer_error':str(e)}
    proof={'label':label,'token':token,'byte_exact':raw==token,'saved_file_label':label+'-saved-file-'+str(attempt),
           'before_frame':before,'frame':name,'native_window':after,'realtime':time.time()}
    result['input_proofs'].append(proof);save()
    if raw!=token:raise InputError('Real native editor save/readback failed; no controller replacement')
    return after

def close_reopen(seq):
    c.key('meta_l-q');time.sleep(1)
    frame,_,_=screen('normal-close-'+str(seq))
    obs=observe('normal-close-native-'+str(seq))
    closed=not any(not w.get('closed') and 'cosmicedit' in w.get('app_id','').lower().replace('-','') for w in obs['windows'])
    result['normal_closes'].append({'closed':closed,'frame':frame,'native':obs})
    if not closed:raise RuntimeError('Normal native close did not close editor; retained exact screen')
    launch_editor(seq);canary('reopened-'+str(seq))

def pointer_move(x,y):
    c.q.call('input-send-event',{'events':[{'type':'abs','data':{'axis':'x','value':round(x*32767/1280)}},{'type':'abs','data':{'axis':'y','value':round(y*32767/800)}}]})

def drag(start,end):
    pointer_move(*start);c.q.call('input-send-event',{'events':[{'type':'btn','data':{'button':'left','down':True}}]});time.sleep(.2)
    for i in range(1,11):pointer_move(round(start[0]+(end[0]-start[0])*i/10),round(start[1]+(end[1]-start[1])*i/10));time.sleep(.06)
    c.q.call('input-send-event',{'events':[{'type':'btn','data':{'button':'left','down':False}}]});time.sleep(1)

def preceding_sequence():
    w=editor_window('preceding-before')
    if w.get('geometry_source')!='native protocol':
        result['preceding_unqualified']='Native geometry absent; screenshot focus cannot qualify resize sequence';save();return
    c.key('meta_l-m');time.sleep(1);maximized=editor_window('preceding-maximized')
    c.key('meta_l-m');time.sleep(1);restored=editor_window('preceding-restored')
    g=restored['geometry'];start=(g['x']+min(180,g['width']//3),g['y']+20)
    drag(start,(max(25,min(1200,start[0]+45)),max(65,min(650,start[1]+20))))
    moved=editor_window('preceding-old-header-target');g=moved['geometry']
    corner=(min(1277,g['x']+g['width']-2),min(795,g['y']+g['height']-2))
    drag(corner,(max(g['x']+400,corner[0]-30),max(g['y']+250,corner[1]-20)))
    resized=editor_window('preceding-resized');screen('preceding-complete')
    result['preceding_sequence']={'initial':w,'maximized':maximized,'restored':restored,'old_header_target':moved,'resized':resized,
                                  'note':'Exact R3 seq9900 order; old header drag may open View menu and is not qualified as movement'}
    save()

def overview(label):
    native_readback(label+'-native',(ev/'native-requested.ron').read_text())
    before=identities(label+'-identity-before');initial=compositor(before)
    if public and [x for x in before if x['exe']=='/usr/bin/halcyon' and '--settings' not in x['args']]!=result['public_main_identity']:
        raise ProofError('Actual public desktop main changed or disappeared before trigger')
    workspace_before=[x for x in before if x['exe']=='/usr/bin/cosmic-workspaces']
    native_before=journal(label+'-journal-before')
    if native_before['compositor_panics'] or native_before['workspaces_panics']:
        result['native_failure_observed']=True
        result['triggers'].append({'label':label,'triggered':False,'native_failure':True,'reason':'Native compositor/workspaces panic already present before this trigger','journal_before':native_before})
        save();return False
    screen(label+'-before-overview')
    result['trigger_started']=True;save()
    started=time.time();c.key('meta_l-w');time.sleep(1);frame,_,_=screen(label+'-overview')
    time.sleep(2)
    after=identities(label+'-identity-after');comps=[x for x in after if x['exe']=='/usr/bin/cosmic-comp']
    native_after=journal(label+'-journal-after')
    stable=len(comps)==1 and comps[0]==initial
    # Full record equality includes PID/starttime/ELF/args, never merely socket health.
    workspace_after=[x for x in after if x['exe']=='/usr/bin/cosmic-workspaces']
    workspace_stable=len(workspace_before)==1 and workspace_after==workspace_before
    main_after=[x for x in after if x['exe']=='/usr/bin/halcyon' and '--settings' not in x['args']]
    main_stable=not public or main_after==result['public_main_identity']
    failed=not stable or not workspace_stable or not main_stable or bool(native_after['compositor_panics'] or native_after['workspaces_panics'])
    proof={'label':label,'trigger_realtime':started,'elapsed_after_treatment_sec':started-result['treatment_applied_realtime'],
           'frame':frame,'public_main_identity_stable':main_stable,'public_main_after':main_after,'workspaces_before':workspace_before,'workspaces_after':workspace_after,'workspaces_identity_stable':workspace_stable,'overview_visibility_requires_frame_review':True,'compositor_before':initial,'compositor_after':comps,'same_identity':stable,
           'journal_before':native_before,'journal_after':native_after,'native_failure':failed}
    result['triggers'].append(proof);result['native_failure_observed']|=failed;save()
    if failed:screen(label+'-native-failure')
    if not stable or native_after['compositor_panics']:
        proof['after_canary']='not attempted: actual compositor death/restart/panic';save();return False
    # A failed Workspaces client does not erase separate real editor input.
    c.key('esc');time.sleep(1);canary(label+'-after-overview')
    proof['after_canary']='real fresh native typing/save/readback passed';save()
    return not failed

def profile_public():
    command('launch-public-setup','systemd-run --user --collect --unit=blur-setup /usr/bin/halcyon --settings setup',True);time.sleep(6)
    _,path,_=screen('public-look-before')
    try:detail=validate_samples(Pixels(path),CAL['wizard_samples'])
    except ValueError:
        detail=validate_samples(Pixels(path),CAL['welcome_samples']);c.click(345,451);time.sleep(2)
        _,path,_=screen('public-look-grid');detail=validate_samples(Pixels(path),CAL['wizard_samples'])
    dx,dy=detail['offset'];c.click(390+dx,336+dy);time.sleep(3);screen('public-frosted-selected')
    command('stop-public-setup','systemctl --user stop blur-setup.service',True)
    conf=json.loads(command('public-selected-config','cat /home/probe/.config/halcyon/config.json'))
    if (conf.get('appearance',{}).get('family'),conf.get('theme'))!=('frosted','ocean'):
        raise ProofError('Public Frosted/ocean identity mismatch')
    result['profile']=conf
    command('start-public','halcyon-ctl start',True);time.sleep(8)
    rows=identities('public-identity')
    if not any(r['exe']=='/usr/bin/halcyon' and r['sha256']==MAIN for r in rows):
        raise ProofError('Actual public main ELF missing')
    result['public_main_identity']=[r for r in rows if r['exe']=='/usr/bin/halcyon' and '--settings' not in r['args']];save()

def behavior():
    if not result.get('fixture_created'):
        command('create-empty-disposable-document','test ! -e /home/probe/lockup-input.txt && install -o 1000 -g 1000 -m 600 /dev/null /home/probe/lockup-input.txt')
        result['fixture_created']=True;save()
    if not result.get('editor_launched'):launch_editor(0)
    canary('before-first-overview');result['preflight_passed']=True;save()
    if variant=='preceding':preceding_sequence()
    overview(variant+'-first-overview')
    result['complete']=True

def main():
    if recover:
        c.q=c.Channel('qmp.sock',True)
        if result.get('trigger_started'):raise ProofError('Never replay a trigger in a used overlay')
        try:verify_media(c.q.call('query-block'),public)
        except ValueError as e:raise ProofError(str(e))
        compositor(identities('recovery-identity'))
        treatment(rechecking=True);behavior();return
    c.wait_agent();c.q=c.Channel('qmp.sock',True);time.sleep(15)
    blocks=c.q.call('query-block');(ev/'qmp-blocks.json').write_text(json.dumps(blocks,indent=2)+'\n')
    try:verify_media(blocks,public)
    except ValueError as e:raise ProofError(str(e))
    c.login('native-login');c.capture('baseline');install_helpers()
    command('native-environment','uname -a; ip -brief addr; ip route; dpkg-query -W cosmic-settings cosmic-comp cosmic-edit; sha256sum /usr/bin/cosmic-settings /usr/bin/cosmic-comp /usr/bin/cosmic-edit; /usr/bin/cosmic-settings appearance --help')
    if public:
        command('mount-public-media','candidate_device=$(blkid -L HALCYON_INPUTS) && mkdir -p /mnt/candidate && mount -o ro "$candidate_device" /mnt/candidate && cd /mnt/candidate && sha256sum -c SHA256SUMS')
        h.install('public');profile_public()
    else:
        command('stock-product-absence','test ! -e /usr/bin/halcyon && test ! -e /home/probe/.config/halcyon')
        command('stock-product-package-status','dpkg-query -W -f="${db:Status-Abbrev} ${binary:Package}\\n" cosmic-ext-halcyon cosmic-ext-halcyon-islands',required=False)
    treatment();behavior()

try:
    main()
except Exception as e:
    category='input' if isinstance(e,InputError) else 'proof' if isinstance(e,ProofError) else 'harness'
    result['setup_error']=str(e);result['error_category']=category
    result['errors'].append({'time':time.time(),'category':category,'error':str(e),'recovery':recover});traceback.print_exc()
    try:screen('setup-or-input-failure')
    except Exception:traceback.print_exc()
finally:
    try:
        result['final_identities']=identities('final-identities'+('-recovery' if recover else ''))
        result['final_journal']=journal('full-native-journal'+('-recovery' if recover else ''))
        if result['final_journal']['compositor_panics'] or result['final_journal']['workspaces_panics']:
            result['native_failure_observed']=True
            if result.get('preflight_passed') and result.get('error_category')!='input':result['complete']=True
        h.snapshot('final-config'+('-recovery' if recover else ''));c.capture('final'+('-recovery' if recover else ''))
    except Exception as e:result['final_collection_error']=str(e)
    result['repair_eligible']=bool(result.get('error_category')=='harness' and result.get('configuration_applied') and not result.get('trigger_started') and not result['native_failure_observed'])
    result['finished_realtime']=time.time();save()
sys.exit(0 if result['complete'] and not result.get('setup_error') else 2)

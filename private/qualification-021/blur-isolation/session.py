#!/usr/bin/python3
"""Immediate native overview/blur comparison. No product patch or synthetic save."""
import base64, hashlib, json, pathlib, re, shlex, sys, time, traceback
SRC = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(SRC.parent / 'theme'))
sys.path.insert(0, str(SRC.parent / 'lockup'))
import theme_session as h  # Definitions-only import; exercise.py is never imported.
from calibrated_controls import Pixels, validate_samples
from configuration import FLAGS, fields, change_flags, verify_media, validate_reexport
c = h.c
work, ev = c.work, c.ev
context = sys.argv[4]
if context not in ['stock-baseline','stock-native-blur','public-frosted','public-native-off']:
    raise ValueError('Unknown isolation context')
public = context.startswith('public-')
result = {'context': context, 'public': public, 'started_realtime': time.time(),
          'input_proofs': [], 'triggers': [], 'normal_closes': [], 'setup_error': None,
          'native_failure_observed': False, 'complete': False, 'injected_mode_events': False}
CAL = json.loads((SRC.parent/'theme/followup/control-calibration.json').read_text())
EDITOR = '61f32f20df9c4da800b0df8054cb0b7ec6c65ccd4b9d11679f7f6c7303dc1149'
MAIN = '0abb0f393b14ba0a5ddf791a719c391dfa9a8a04e39196a1caddc8ff7028a01f'
COMP = '2399b768cc6a4599a6495d0865cf4dc6807cfee2c6e46b3d7466a800f903be5e'

def save():
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
    observed = observe(label)
    hits = [w for w in observed['windows'] if not w.get('closed') and 'cosmicedit' in w.get('app_id','').lower().replace('-','')]
    if len(hits) != 1 or 'geometry' not in hits[0]:
        screen(label+'-metadata-unqualified')
        raise RuntimeError('Editor metadata unavailable/ambiguous; not evidence that app is unmapped')
    return hits[0]

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
        raise RuntimeError('Pinned compositor missing or wrong ELF')
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
    fields(raw)
    return raw

def treatment():
    before_snapshot=h.snapshot('treatment-before')
    before=export('native-builder-before')
    original=fields(before)
    expected='true' if public else 'false'
    if any(original[f]!=expected for f in FLAGS):
        raise RuntimeError('Initial native blur differs from R3 treatment')
    desired=before
    change=None
    if context in ['stock-native-blur','public-native-off']:
        enabled=context=='stock-native-blur'
        desired,change=change_flags(before,enabled)
        encoded=base64.b64encode(desired.encode()).decode()
        command('stage-native-import','umask 077; echo '+shlex.quote(encoded)+' | base64 -d > /var/tmp/blur-treatment.ron',True)
        command('native-import','/usr/bin/cosmic-settings appearance import /var/tmp/blur-treatment.ron',True)
        time.sleep(6)
    after=export('native-builder-after')
    after_snapshot=h.snapshot('treatment-after')
    raw=after_snapshot['files']
    if change is None:
        if after!=desired:raise RuntimeError('Untouched export changed')
        projection={'kind':'byte-exact untouched export'}
    else:
        prefix='.config/cosmic/com.system76.CosmicTheme.Dark'
        tint=bytes.fromhex(raw[prefix+'.Builder/v2/text_tint']['hex']).decode()
        background=bytes.fromhex(raw[prefix+'/v2/background']['hex']).decode()
        projection=validate_reexport(desired,after,tint,background)
    wanted='true' if context in ['stock-native-blur','public-frosted'] else 'false'
    if context!='stock-baseline':
        for f in FLAGS:
            path='.config/cosmic/com.system76.CosmicTheme.Dark/v2/'+f
            actual=bytes.fromhex(after_snapshot['files'][path]['hex']).decode().strip()
            if actual!=wanted:raise RuntimeError('Native generated theme flag mismatch: '+f)
    result['treatment']={'exported_before':original,'exported_after':fields(after),
                         'exact_reexport_matches_input':after==desired,'native_export_projection':projection,'change':change,
                         'before_builder_sha256':hashlib.sha256(before.encode()).hexdigest(),
                         'after_builder_sha256':hashlib.sha256(after.encode()).hexdigest(),
                         'widget_blur_requests_retained':public,
                         'active_variant':'Dark; verified in native exported builder'}
    result['treatment_applied_realtime']=time.time()
    screen('native-treatment-visible')

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
        raise RuntimeError('Initial native forking service is not active/successful')
    rows=identities('editor-identity-'+str(seq))
    if not any(r['exe']=='/usr/bin/cosmic-edit' and r['sha256']==EDITOR for r in rows):
        raise RuntimeError('Actual native editor ELF missing')
    return editor_window('editor-open-'+str(seq))

def canary(label):
    w=editor_window(label+'-before');g=w['geometry']
    c.click(max(40,min(1240,g['x']+g['width']//2)),max(100,min(670,g['y']+g['height']//2)))
    token='b'+context.replace('-','')+str(len(result['input_proofs'])+1).zfill(3)
    before,_,_=screen(label+'-before-input')
    c.key('ctrl-a');c.text(token);c.key('ctrl-s');time.sleep(1)
    deadline=time.monotonic()+5;attempt=0
    while True:
        raw=command(label+'-saved-file-'+str(attempt),'cat /home/probe/lockup-input.txt')
        if raw==token or time.monotonic()>=deadline:break
        time.sleep(.5);attempt+=1
    name,_,_=screen(label+'-input')
    after=editor_window(label+'-after')
    proof={'label':label,'token':token,'byte_exact':raw==token,'saved_file_label':label+'-saved-file-'+str(attempt),
           'before_frame':before,'frame':name,'native_window':after,'realtime':time.time()}
    result['input_proofs'].append(proof);save()
    if raw!=token:raise RuntimeError('Real native editor save/readback failed; no controller replacement')
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
    current=export(label+'-native-builder')
    if hashlib.sha256(current.encode()).hexdigest()!=result['treatment']['after_builder_sha256']:
        raise RuntimeError('Native treatment changed before overview; comparison unqualified')
    before=identities(label+'-identity-before');initial=compositor(before)
    workspace_before=[x for x in before if x['exe']=='/usr/bin/cosmic-workspaces']
    native_before=journal(label+'-journal-before')
    if native_before['compositor_panics'] or native_before['workspaces_panics']:
        result['native_failure_observed']=True
        result['triggers'].append({'label':label,'triggered':False,'native_failure':True,'reason':'Native compositor/workspaces panic already present before this trigger','journal_before':native_before})
        save();return False
    screen(label+'-before-overview')
    started=time.time();c.key('meta_l-w');time.sleep(1);frame,_,_=screen(label+'-overview')
    time.sleep(2)
    after=identities(label+'-identity-after');comps=[x for x in after if x['exe']=='/usr/bin/cosmic-comp']
    native_after=journal(label+'-journal-after')
    stable=len(comps)==1 and comps[0]==initial
    # Full record equality includes PID/starttime/ELF/args, never merely socket health.
    workspace_after=[x for x in after if x['exe']=='/usr/bin/cosmic-workspaces']
    workspace_stable=len(workspace_before)==1 and workspace_after==workspace_before
    failed=not stable or not workspace_stable or bool(native_after['compositor_panics'] or native_after['workspaces_panics'])
    proof={'label':label,'trigger_realtime':started,'elapsed_after_treatment_sec':started-result['treatment_applied_realtime'],
           'frame':frame,'workspaces_before':workspace_before,'workspaces_after':workspace_after,'workspaces_identity_stable':workspace_stable,'overview_visibility_requires_frame_review':True,'compositor_before':initial,'compositor_after':comps,'same_identity':stable,
           'journal_before':native_before,'journal_after':native_after,'native_failure':failed}
    result['triggers'].append(proof);result['native_failure_observed']|=failed;save()
    if failed:
        screen(label+'-native-failure')
        proof['after_canary']='not attempted: actual compositor death/restart/panic; cannot qualify input'
        save();return False
    c.key('esc');time.sleep(1);canary(label+'-after-overview');close_reopen(len(result['triggers']))
    proof['after_canary']='real exact save, normal close, reopen and another exact save passed';save()
    return True

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
        raise RuntimeError('Public Frosted/ocean identity mismatch')
    result['profile']=conf
    command('start-public','halcyon-ctl start',True);time.sleep(8)
    rows=identities('public-identity')
    if not any(r['exe']=='/usr/bin/halcyon' and r['sha256']==MAIN for r in rows):
        raise RuntimeError('Actual public main ELF missing')

def main():
    c.wait_agent();c.q=c.Channel('qmp.sock',True);time.sleep(15)
    blocks=c.q.call('query-block');(ev/'qmp-blocks.json').write_text(json.dumps(blocks,indent=2)+'\n')
    verify_media(blocks,public)
    c.login('native-login');c.capture('baseline');install_helpers()
    command('native-environment','uname -a; ip -brief addr; ip route; dpkg-query -W cosmic-settings cosmic-comp cosmic-edit; sha256sum /usr/bin/cosmic-settings /usr/bin/cosmic-comp /usr/bin/cosmic-edit; /usr/bin/cosmic-settings appearance --help')
    if public:
        command('mount-public-media','candidate_device=$(blkid -L HALCYON_INPUTS) && mkdir -p /mnt/candidate && mount -o ro "$candidate_device" /mnt/candidate && cd /mnt/candidate && sha256sum -c SHA256SUMS')
        h.install('public');profile_public()
    else:
        command('stock-product-absence','test ! -e /usr/bin/halcyon && test ! -e /home/probe/.config/halcyon')
        command('stock-product-package-status','dpkg-query -W -f="${db:Status-Abbrev} ${binary:Package}\\n" cosmic-ext-halcyon cosmic-ext-halcyon-islands',required=False)
    treatment()
    command('create-empty-disposable-document','install -o 1000 -g 1000 -m 600 /dev/null /home/probe/lockup-input.txt')
    launch_editor(0);canary('before-first-overview')
    result['preflight_passed']=True;save()
    if overview('immediate'):
        preceding_sequence();overview('after-preserved-sequence')
    result['complete']=True

try:
    main()
except Exception as e:
    result['setup_error']=str(e);traceback.print_exc()
    try:screen('setup-or-input-failure')
    except Exception:traceback.print_exc()
finally:
    try:
        result['final_identities']=identities('final-identities')
        result['final_journal']=journal('full-native-journal')
        if result['final_journal']['compositor_panics'] or result['final_journal']['workspaces_panics']:
            result['native_failure_observed']=True
            if result.get('preflight_passed'):
                result['complete']=True
                if result['setup_error']:
                    result['downstream_error_after_native_failure']=result['setup_error'];result['setup_error']=None
        h.snapshot('final-config');c.capture('final')
    except Exception as e:result['final_collection_error']=str(e)
    result['finished_realtime']=time.time();save()
sys.exit(0 if result['complete'] else 2)

"""Actual launcher + real ELF/procfs lifecycle. NOT native login qualification."""
import copy
import json
import os
from pathlib import Path
import shutil
import signal
import socket
import subprocess
import sys
import tempfile
import time
import unittest

from bundle_contract import canonical, cohort_id, digest, identity, runtime_id, valid_catalog
import install
import refresh
import runtime_identity
from test_generation_contract import bundle_fixture, catalog_fixture

HERE = Path(__file__).resolve().parent

RUNNER = r'''
import ctypes,json,os,signal,subprocess,sys,threading,time
from pathlib import Path
sys.path.insert(0,sys.argv[1])
import panel_launcher as launcher
import runtime_identity as runtime
import install_status
from bundle_contract import classify_compatibility
cfg=json.loads(Path(sys.argv[2]).read_bytes())
# Test-only subreaper: collect synthetic orphan grandchildren after killpg.
ctypes.CDLL(None).prctl(36,1,0,0,0)
real_installed=runtime.installed_identity
runtime.installed_identity=lambda catalog: real_installed(catalog,Path(cfg['installed_root']),cfg['versions'],cfg['system'])
launcher.STOCK=Path(cfg['stock']);launcher.READY_SECONDS=1.2
if cfg['mode']=='slow-measurement':
    prepare=launcher.prepare_launch
    def slow_prepare(bundle):
        admitted=prepare(bundle)
        if admitted is None:return None
        def slow(child):
            time.sleep(10);return admitted(child)
        return slow
    launcher.prepare_launch=slow_prepare
real_wait=launcher.wait_ready
foreign=[]
def observed_wait(child,done,wedged,admission=None):
    if cfg['mode']=='foreign':
        # An unrelated same-session actor cannot substitute for the host's child.
        foreign.append(subprocess.Popen([cfg['applet'],'90']))
    owner=runtime.host_identity(cfg['binary_sha256']);deadline=time.monotonic()+1
    while owner is None and time.monotonic()<deadline:
        time.sleep(.005);owner=runtime.host_identity(cfg['binary_sha256'])
    if cfg['mode']=='core-exit':os.kill(cfg['core_pid'],signal.SIGTERM)
    before=runtime.running_identity(cfg['catalog'],host=owner)
    Path(cfg['observations']+'-early').write_text(json.dumps({'ipc_owner_verified':owner is not None,'runtime':before,'monotonic_ns':time.monotonic_ns()}))
    Path(os.environ['FIXTURE_ALLOW_SPAWN']).touch()
    ready=real_wait(child,done,wedged,admission)
    if ready:
        owner=runtime.host_identity(cfg['binary_sha256'])
        measured=runtime.running_identity(cfg['catalog'],host=owner)
        state=install_status.detect(Path(cfg['home']),Path(cfg['library']),True,cfg['versions'],
            installed=runtime.installed_identity(cfg['catalog']),running=measured,host=owner is not None)
        Path(cfg['observations']).write_text(json.dumps({'ready':ready,'runtime':measured,'status':state,'pid':child.pid,'monotonic_ns':time.monotonic_ns()}))
    else:
        Path(cfg['observations']).write_text(json.dumps({'ready':False,'monotonic_ns':time.monotonic_ns()}))
    return ready
launcher.wait_ready=observed_wait
try:
    result=launcher.main(bundle=Path(cfg['bundle']),pointer=cfg['pointer'])
    Path(cfg['exit']).write_text(json.dumps({'result':result,'foreign_survived':[p.poll() is None for p in foreign]}))
finally:
    for child in foreign:
        if child.poll() is None:child.terminate()
        child.wait()
    deadline=time.monotonic()+2
    while time.monotonic()<deadline:
        try:
            pid,_=os.waitpid(-1,os.WNOHANG)
            if pid==0:time.sleep(.01)
        except ChildProcessError:break
sys.exit(result)
'''


class LaunchLifecycle(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.build=tempfile.TemporaryDirectory()
        cls.binary=Path(cls.build.name)/'host'
        subprocess.run(['cc','-O2','-Wall','-Wextra','-Werror',str(HERE/'fixtures/lifecycle_host.c'),'-o',str(cls.binary)],check=True)
        cls.binary.chmod(0o755)

    @classmethod
    def tearDownClass(cls):
        cls.build.cleanup()

    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup)
        self.root=Path(self.tmp.name);self.home=self.root/'home';self.home.mkdir()
        self.installed_root=self.root/'installed';(self.installed_root/'usr/bin').mkdir(parents=True)
        self.runtime=self.root/'runtime';self.runtime.mkdir()
        self.events=self.root/'events';self.events.touch()
        self.env={**os.environ,'HOME':str(self.home),'XDG_DATA_HOME':str(self.home/'.local/share'),
            'XDG_STATE_HOME':str(self.home/'.local/state'),'XDG_CONFIG_HOME':str(self.home/'.config'),
            'XDG_RUNTIME_DIR':str(self.runtime),'XDG_SESSION_ID':'lifecycle-'+self.root.name,
            'WAYLAND_DISPLAY':'fixture-compositor','FIXTURE_EVENTS':str(self.events),'FIXTURE_ALLOW_SPAWN':str(self.root/'allow-spawn')}
        # No inherited real session descriptors can reach the synthetic test.
        for name in ('WAYLAND_SOCKET','PANEL_NOTIFICATIONS_FD','COSMIC_SESSION_SOCK'):
            self.env.pop(name,None)
        self.display=socket.socket(socket.AF_UNIX);self.display.bind(str(self.runtime/'fixture-compositor'));self.display.listen(16)
        self.addCleanup(self.display.close)
        self.children=[];self.runners=[];self.addCleanup(self.cleanup)
        self.catalog=catalog_fixture();self.row=self.catalog['cohorts'][0]
        self.row['runtime']['processes']={}
        for name in ('cosmic-comp','cosmic-session','cosmic-settings-daemon','cosmic-applet-audio'):
            path=self.installed_root/'usr/bin'/name;shutil.copy2('/usr/bin/sleep',path)
            child=self.start_actor(path)
            observed=runtime_identity._process(Path('/proc')/str(child.pid))
            self.row['runtime']['processes'][name]={**{k:observed[k] for k in ('executable','sha256','mapped')},
                'count':1,'lifecycle':'panel-child' if name=='cosmic-applet-audio' else 'session'}
            self.row['packages'][name]=dict(version='fixture-1',sha256='b'*64,files={'/usr/bin/'+name:digest(path)})
            if name=='cosmic-applet-audio':child.terminate();child.wait()
        # The sample actor used to record maps is gone before any launcher call.
        self.applet=self.installed_root/'usr/bin/cosmic-applet-audio'
        self.stock=self.root/'stock';shutil.copy2(self.binary,self.stock)
        stock_installed=self.installed_root/'usr/bin/cosmic-panel';shutil.copy2(self.binary,stock_installed)
        self.row['packages']['cosmic-panel']=dict(version='fixture-1',sha256='b'*64,files={'/usr/bin/cosmic-panel':digest(stock_installed)})
        self.other=self.root/'other/cosmic-applet-audio';self.other.parent.mkdir();self.other.write_bytes(self.applet.read_bytes()+b'unknown code');self.other.chmod(0o755)
        self.replacement=self.root/'replacement';shutil.copy2(self.other,self.replacement)
        self.library=self.root/'library'
        self.manifest,_=bundle_fixture(self.library,'0.2.1~lifecycle')
        shutil.copy2(self.binary,self.library/'cosmic-panel-islands')
        self.finalize()

    def tearDown(self):
        evidence=os.environ.get('HALCYON_LIFECYCLE_EVIDENCE')
        if evidence:
            destination=Path(evidence)/self._testMethodName
            destination.mkdir(parents=True,exist_ok=False)
            for name in ('events','observations','observations-early','runner.log','exit','runner.json'):
                if (self.root/name).exists():shutil.copy2(self.root/name,destination/name)
            (destination/'NOT-NATIVE.json').write_bytes(canonical(dict(kind='synthetic real-procfs and actual launcher regression',native_qualification=False)))

    def cleanup(self):
        for runner in self.runners:
            if runner.poll() is None:runner.terminate()
            try:runner.wait(timeout=4)
            except subprocess.TimeoutExpired:runner.kill();runner.wait()
        for child in self.children:
            if child.poll() is None:child.terminate()
            child.wait()

    def start_actor(self,path):
        child=subprocess.Popen([str(path),'90'],env=self.env);self.children.append(child)
        deadline=time.monotonic()+2
        while time.monotonic()<deadline:
            try:
                if os.readlink(Path('/proc')/str(child.pid)/'exe')==str(path):
                    runtime_identity._process(Path('/proc')/str(child.pid));return child
            except (OSError,ValueError):pass
            time.sleep(.002)
        self.fail('fixture actor failed to exec')

    def finalize(self):
        self.row['processes']={n:p['executable'] for n,p in self.row['runtime']['processes'].items()}
        self.row['id']=cohort_id(self.row)
        self.row['runtime_compatibility']=[dict(id=runtime_id(self.row['runtime']),evidence_id=self.row['evidence']['id'],reason='Synthetic lifecycle fixture only, not native evidence')]
        files=self.manifest['files'];files['cosmic-panel-islands']['sha256']=digest(self.library/'cosmic-panel-islands')
        self.manifest['binary_sha256']=files['cosmic-panel-islands']['sha256']
        self.manifest['payload_sha256']=identity({k:v for k,v in files.items() if k!='catalog.json'})
        self.row['payload_sha256']=self.manifest['payload_sha256'];valid_catalog(self.catalog)
        (self.library/'catalog.json').write_bytes(canonical(self.catalog));files['catalog.json']['sha256']=digest(self.library/'catalog.json')
        (self.library/'companion-manifest.json').write_bytes(canonical(self.manifest));(self.library/'companion-manifest.json').chmod(0o644)
        self.versions={n:p['version'] for n,p in self.row['packages'].items()}
        self.system={k:self.row[k] for k in ('distribution','suite','architecture')}

    def launch(self,mode='success'):
        (self.home/'installed-identity.json').write_bytes(canonical(dict(verified=True,cohort=self.row['id'])))
        with refresh.locked(self.home):
            staged=refresh.stage_locked(self.home,self.library,fixture=True,enable=True,session='previous-login')
        self.assertEqual(staged['state'],'staged');self.assertEqual(staged['runtime_readiness'],'not_measured')
        bundle,_,_,pointer=refresh.selected_generation(self.home)
        cfg=dict(mode=mode,core_pid=self.children[0].pid,installed_root=str(self.installed_root),versions=self.versions,system=self.system,
            stock=str(self.stock),catalog=self.catalog,home=str(self.home),library=str(self.library),bundle=str(bundle),pointer=pointer,
            binary_sha256=self.manifest['binary_sha256'],applet=str(self.applet),observations=str(self.root/'observations'),exit=str(self.root/'exit'))
        config=self.root/'runner.json';config.write_bytes(canonical(cfg));runner=self.root/'runner.py';runner.write_text(RUNNER)
        self.log=(self.root/'runner.log').open('wb');self.addCleanup(self.log.close)
        env={**self.env,'FIXTURE_MODE':mode,'FIXTURE_APPLET':str(self.applet),'FIXTURE_OTHER_APPLET':str(self.other),'FIXTURE_REPLACEMENT':str(self.replacement)}
        child=subprocess.Popen([sys.executable,str(runner),str(HERE),str(config)],env=env,stdout=self.log,stderr=subprocess.STDOUT)
        self.runners.append(child);return child

    def await_file(self,name,child):
        deadline=time.monotonic()+6
        while time.monotonic()<deadline:
            path=self.root/name
            if path.exists():return json.loads(path.read_bytes())
            if child.poll() is not None:break
            time.sleep(.01)
        self.fail((self.root/'runner.log').read_text())

    def events_named(self,name):
        return [int(line.split()[1]) for line in self.events.read_text().splitlines() if line.split()[0]==name]

    def assert_reaped(self):
        for pid in self.events_named('host')+self.events_named('applet'):
            self.assertFalse((Path('/proc')/str(pid)).exists(),f'fixture PID {pid} not reaped')

    def test_core_only_login_spawns_owned_applet_then_ready_and_clean_teardown(self):
        self.successful_login()

    def successful_login(self,mode='success',count=1):
        pre=runtime_identity.running_identity(self.catalog,session=self.env['XDG_SESSION_ID'],phase='prelaunch')
        self.assertTrue(pre['verified']);self.assertEqual(pre['phase'],'prelaunch')
        child=self.launch(mode);observed=self.await_file('observations',child)
        self.assertTrue(observed['ready'],(self.root/'runner.log').read_text())
        early=json.loads((self.root/'observations-early').read_bytes())
        self.assertTrue(early['ipc_owner_verified']);self.assertFalse(early['runtime']['verified'])
        self.assertEqual(observed['status']['status'],'Ready')
        self.assertEqual(observed['runtime']['phase'],'running')
        self.assertEqual(len(self.events_named('host')),1);self.assertEqual(len(self.events_named('applet')),count)
        self.assertEqual(len(self.events_named('probe')),1)
        for line in self.events.read_text().splitlines():
            if line.startswith('applet '):
                self.assertLess(early['monotonic_ns'],int(line.split()[2]))
                self.assertLess(int(line.split()[2]),observed['monotonic_ns'])
        # An arbitrary prelaunch result cannot become running readiness in status.
        import install_status
        from unittest.mock import patch
        with patch.dict(os.environ,self.env):
            status=install_status.detect(self.home,self.library,True,self.versions,installed=dict(verified=True,cohort=self.row['id']),running=pre,host=True)
        self.assertNotEqual(status['status'],'Ready');self.assertEqual(status['running_profiles'],[])
        child.terminate();self.assertEqual(child.wait(timeout=5),0)
        self.assertEqual(self.events_named('stock'),[])
        self.assertFalse((self.home/'.local/state/desktop-cards-panel/failed').exists())
        self.assert_reaped()
        # Disable keeps the existing guarded restore chain.
        with refresh.locked(self.home):
            for path,_ in refresh.records(self.home):install.restore(path)
        self.assertFalse((self.home/refresh.SELECTOR).exists())

    def test_missing_required_child_times_out_before_ready(self):
        self.failure('missing')

    def failure(self,mode):
        child=self.launch(mode);self.assertEqual(child.wait(timeout=6),0,(self.root/'runner.log').read_text())
        self.assertFalse(json.loads((self.root/'observations').read_bytes())['ready'])
        self.assertEqual(len(self.events_named('host')),1);self.assertEqual(len(self.events_named('stock')),1)
        self.assertTrue((self.home/'.local/state/desktop-cards-panel/failed').exists())
        self.assert_reaped()
        if mode=='foreign':self.assertEqual(json.loads((self.root/'exit').read_bytes())['foreign_survived'],[True])

    def test_declared_two_children_can_become_ready(self):
        self.row['runtime']['processes']['cosmic-applet-audio']['count']=2;self.finalize()
        self.successful_login('excess',2)

    def test_required_core_exit_after_launch_is_not_ignored(self):self.failure('core-exit')

    def test_slow_runtime_measurement_cannot_extend_readiness_deadline(self):
        started=time.monotonic();self.failure('slow-measurement')
        self.assertLess(time.monotonic()-started,4)

    def test_mismatched_child(self):self.failure('mismatch')
    def test_package_replaced_between_prelaunch_and_spawn(self):self.failure('replace')
    def test_wrong_session_child(self):self.failure('wrong-session')
    def test_excess_child_count(self):self.failure('excess')
    def test_matching_unowned_applet_cannot_supply_readiness(self):self.failure('foreign')

    def blocked(self):
        child=self.launch();self.assertEqual(child.wait(timeout=5),0,(self.root/'runner.log').read_text())
        self.assertEqual(self.events_named('host'),[]);self.assertEqual(self.events_named('probe'),[])
        self.assertEqual(len(self.events_named('stock')),1)
        self.assertFalse((self.home/'.local/state/desktop-cards-panel/failed').exists())

    def test_leftover_matching_applet_blocks_before_starting_any_host(self):
        foreign=self.start_actor(self.applet);self.blocked();self.assertIsNone(foreign.poll())

    def test_old_running_applet_with_new_installed_bytes_blocks_prelaunch(self):
        foreign=self.start_actor(self.applet)
        os.replace(self.replacement,self.applet)
        self.row['runtime']['processes']['cosmic-applet-audio']['sha256']=digest(self.applet)
        self.row['runtime']['processes']['cosmic-applet-audio']['mapped'][str(self.applet)]=digest(self.applet)
        self.row['packages']['cosmic-applet-audio']['files']['/usr/bin/cosmic-applet-audio']=digest(self.applet)
        self.finalize();self.blocked();self.assertIsNone(foreign.poll())

    def test_installed_applet_bytes_mismatch_blocks_prelaunch(self):
        os.replace(self.replacement,self.applet);self.blocked()

    def test_missing_required_core_is_not_prelaunch_eligibility(self):
        self.children[0].terminate();self.children[0].wait();self.blocked()

    def test_schema_rejects_panel_cycle_and_core_only_equivalence(self):
        bad=copy.deepcopy(self.catalog);row=bad['cohorts'][0]
        row['runtime']['processes']['cosmic-panel-islands']=row['runtime']['processes'].pop('cosmic-applet-audio')
        with self.assertRaisesRegex(ValueError,'host_is_not_a_dependency'):valid_catalog(bad)
        bad=copy.deepcopy(self.catalog);other=copy.deepcopy(bad['cohorts'][0])
        other['runtime']['processes'].pop('cosmic-applet-audio');other['processes'].pop('cosmic-applet-audio')
        other['id']=cohort_id(other);other['runtime_compatibility'][0]['id']=runtime_id(other['runtime']);bad['cohorts'].append(other)
        bad['cohorts'][0]['runtime_compatibility'].append(other['runtime_compatibility'][0])
        with self.assertRaisesRegex(ValueError,'equivalence_lifecycle'):valid_catalog(bad)

if __name__=='__main__':unittest.main()

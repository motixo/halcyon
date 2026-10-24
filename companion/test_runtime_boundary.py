"""Runtime boundary regressions. Every policy here is synthetic, NOT qualified."""
import copy
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import tempfile
import time
import unittest
from unittest.mock import patch

import bundle_contract as b
from runtime_identity import FileMeasurement, installed_identity
from runtime_target import target_id, valid_target
from verification import Worker, bounded
from test_generation_contract import catalog_fixture, bundle_fixture, runtime_fixture, bind_target, installed, measured, FIXTURE_BINARY_SHA


class Boundary(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory(); self.addCleanup(self.tmp.cleanup);self.root=Path(self.tmp.name)
    def test_outer_owner_death_reaps_probe_and_replacement_dispatches_stock(self):
        # Separate harness owns a subreaper; never alter the test runner/session.
        for name in ('SIGTERM', 'SIGKILL', 'WORKER_KILL'):
            with self.subTest(signal=name):
                result = subprocess.run([os.sys.executable, '-B',
                    str(Path(__file__).with_name('cancellation_probe_fixture.py')), name],
                    capture_output=True, text=True, timeout=10)
                self.assertEqual(result.returncode, 0, result.stdout+result.stderr)
                report = json.loads(result.stdout)
                self.assertIsNone(report['probe'])
                self.assertEqual(report['replacement_dispatch'], ['stock'])

    def installed_fixture(self):
        cat=catalog_fixture();row=cat['cohorts'][0]
        for name,pkg in row['packages'].items():
            path=self.root/'usr/bin'/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(name.encode())
            pkg['files']['/usr/bin/'+name]=b.digest(path)
        runtime_fixture(row);bind_target(row,self.root)
        versions={n:(p['version'],p['architecture']) for n,p in row['runtime_target']['packages'].items()}
        return cat,versions,row['runtime_target']['system']
    def test_complete_closure_unchanged_but_unrelated_secret_and_diversion_never_opened(self):
        cat,versions,system=self.installed_fixture();row=cat['cohorts'][0]
        # Preserve the separate complete archive closure, including the original
        # base-files archive bytes. This is not an installed-image rewrite.
        row['packages']['chrony']=dict(version='1',sha256='e'*64,files={'/usr/share/chrony/chrony.keys':'d'*64})
        row['packages']['base-files']=dict(version='1',sha256='f'*64,files={'/usr/lib/os-release':'c'*64})
        row['id']=b.cohort_id(row);target=row['runtime_target'];target['qualification_cohort_id']=row['id'];target['id']=target_id(target)
        before=b.canonical(row['packages']);protected=self.root/'usr/share/chrony/chrony.keys'
        protected.parent.mkdir(parents=True);protected.write_bytes(b'NEVER READ');protected.chmod(0)
        public=self.root/'usr/lib/os-release';public.parent.mkdir(parents=True);public.write_bytes(b'legitimate Pop public branding')
        original=FileMeasurement.hash;reads=[]
        def recording(measurement,path,*args,**kwargs):reads.append(str(path));return original(measurement,path,*args,**kwargs)
        with patch.object(FileMeasurement,'hash',recording):
            result=installed_identity(cat,self.root,{**versions,'chrony':('future','amd64')},system)
        self.assertTrue(result['verified']);self.assertNotIn('cohort',result)
        self.assertFalse(any('chrony' in p or 'os-release' in p for p in reads))
        self.assertEqual(b.canonical(row['packages']),before)
        manifest=dict(format=b.FORMAT,version=2,verified=True,binary_sha256=FIXTURE_BINARY_SHA,payload_sha256=row['payload_sha256'],upstream=b.CONFIGURATION_SOURCE['upstream'])
        self.assertEqual(b.classify_compatibility(result,measured(row),manifest,cat)['state'],'ready')
        self.assertNotEqual(b.classify_compatibility(dict(cohort=row['id'],verified=True),measured(row),manifest,cat)['state'],'ready')
    def test_required_file_version_architecture_and_loader_change_refuse(self):
        cat,versions,system=self.installed_fixture()
        for replacement in (('new','amd64'),('1.0','arm64')):
            v={**versions,'cosmic-comp':replacement}
            self.assertFalse(installed_identity(cat,self.root,v,system)['verified'])
        with patch.dict(os.environ,LD_PRELOAD='/unqualified/provider'):
            self.assertFalse(installed_identity(cat,self.root,versions,system)['verified'])
        (self.root/'usr/bin/cosmic-comp').write_bytes(b'new required code')
        self.assertFalse(installed_identity(cat,self.root,versions,system)['verified'])
    def test_target_policy_identity_binds_profile_equivalence_and_provenance(self):
        cat,_,_=self.installed_fixture();row=cat['cohorts'][0];old=row['runtime_target']['id']
        for key,value in [('reason','new justification'),('sha256','e'*64),('source_identity','f'*64)]:
            target=copy.deepcopy(row['runtime_target']);next(iter(target['installed_files'].values()))[key]=value
            self.assertNotEqual(target_id(target),old)
        row['runtime']['constraints']={'kernel_release':'other'}
        with self.assertRaisesRegex(ValueError,'binding'):valid_target(row['runtime_target'],row)
    def test_one_catalog_parse_per_operation_and_immutable_capture(self):
        bundle=self.root/'bundle';bundle_fixture(bundle)
        generation=self.root/'generation';shutil.copytree(bundle,generation)
        calls=[];real=b._json_object
        def parse(data):
            value=real(data)
            if value.get('format')==b.CATALOG:calls.append(True)
            return value
        with patch.object(b,'_json_object',parse),b.VerificationContext() as context:
            manifest,cat=b.verify_bundle(bundle,'companion-manifest.json',context=context)
            self.assertIs(b.verify_bundle(bundle,'companion-manifest.json',context=context)[1],cat)
            self.assertIs(b.verify_bundle(generation,'companion-manifest.json',context=context)[1],cat)
            self.assertEqual(len(calls),1)
            with self.assertRaises(TypeError):cat['cohorts'].clear()
            with self.assertRaises(TypeError):cat['cohorts'][0]['packages']['cosmic-comp']['version']='bad'
            path=bundle/'selector.py';raw=path.read_bytes();path.write_bytes(raw+b'X');path.write_bytes(raw)
            with self.assertRaisesRegex(ValueError,'changed'):context.check()
        with b.VerificationContext() as context:b.verify_bundle(bundle,'companion-manifest.json',context=context)
    def test_content_deduplicated_only_with_pinned_descriptor_and_aba_invalidates(self):
        path=self.root/'object';path.write_bytes(b'123456');alias=self.root/'alias';os.link(path,alias)
        with FileMeasurement(root=self.root,max_files=1,max_bytes=6) as phase:
            self.assertEqual(phase.hash(path),phase.hash(alias));self.assertEqual(phase.bytes,6)
            (self.root/'unrelated').write_bytes(b'ignored');phase.check()
            raw=path.read_bytes();path.write_bytes(b'654321');path.write_bytes(raw)
            with self.assertRaisesRegex(ValueError,'changed'):phase.check()
        with FileMeasurement(root=self.root,max_files=1,max_bytes=5) as phase:
            with self.assertRaisesRegex(ValueError,'work_exceeded'):phase.hash(path)
        with FileMeasurement(root=self.root) as phase:
            phase.hash(path);self.assertEqual(phase.bytes,6)
    def test_required_symlink_escape_replacement_and_broken_fail(self):
        path=self.root/'target';path.write_bytes(b'required');link=self.root/'link';link.symlink_to('target')
        with FileMeasurement(root=self.root) as phase:
            phase.hash(phase.resolve(link));link.unlink();link.symlink_to('target')
            with self.assertRaisesRegex(ValueError,'changed'):phase.check()
        link.unlink();link.symlink_to('/usr/bin/python3')
        with FileMeasurement(root=self.root) as phase:
            with self.assertRaisesRegex(ValueError,'root_escape'):phase.resolve(link)
    def test_wired_status_presence_disabled_and_timeout_keep_management_without_io_writes(self):
        import contextlib,io,sys
        import install_status,runtime_identity
        library=self.root/'lib/cosmic-ext-halcyon-islands';bundle_fixture(library)
        home=self.root/'home';home.mkdir();real_path=Path
        class FixturePath(type(Path())):
            def __new__(cls,*args):
                if args==('/usr/lib',):args=(str(self.root/'lib'),)
                return super().__new__(cls,*args)
            def __init__(instance,*args):
                if args==('/usr/lib',):args=(str(self.root/'lib'),)
                super().__init__(*args)
            @classmethod
            def home(cls):return home
        def run(*arguments):
            output=io.StringIO()
            with patch.object(install_status,'Path',FixturePath),patch.object(install_status,'query',return_value='install ok installed'), \
                 patch.object(install_status,'regular',return_value=True),patch.object(sys,'argv',['status',*arguments]),contextlib.redirect_stdout(output):
                install_status.main()
            return [json.loads(line) for line in output.getvalue().splitlines()]
        with patch.object(runtime_identity,'installed_identity',side_effect=AssertionError('disabled must not scan')):
            self.assertEqual(run('--presence')[0]['installation'],'installed')
            self.assertEqual(run()[0]['reason'],'disabled')
        selection=home/'.local/share/cosmic-desktop-cards/companion/selection.json'
        selection.parent.mkdir(parents=True);selection.write_text('{"enabled":true}')
        before={str(p):p.read_bytes() for p in home.rglob('*') if p.is_file()}
        original=__import__('verification').bounded
        with patch.object(runtime_identity,'installed_identity',side_effect=lambda *a,**k:time.sleep(20)), \
             patch('verification.bounded',side_effect=lambda fn,timeout:original(fn,.1)):
            rows=run('--progress')
        self.assertEqual(len(rows),2)
        for row in rows:
            self.assertEqual(row['installation'],'installed');self.assertIsNone(row['compatible'])
            self.assertEqual(row['status'],'Checking compatibility')
        self.assertEqual(rows[-1]['reason'],'verification_timeout')
        self.assertEqual({str(p):p.read_bytes() for p in home.rglob('*') if p.is_file()},before)

    def test_diagnostic_cli_uses_same_cancellable_boundary(self):
        import contextlib,io,sys,runtime_identity
        bundle=self.root/'diagnostic';bundle_fixture(bundle)
        output=io.StringIO();original=__import__('verification').bounded;start=time.monotonic()
        with patch.object(sys,'argv',['measure','--catalog',str(bundle/'catalog.json')]), \
             patch.object(runtime_identity,'installed_identity',side_effect=lambda *a,**k:time.sleep(20)), \
             patch('verification.bounded',side_effect=lambda fn,timeout:original(fn,.08)),contextlib.redirect_stdout(output):
            with self.assertRaises(SystemExit) as stopped:runtime_identity.main()
        self.assertEqual(stopped.exception.code,2);self.assertLess(time.monotonic()-start,.5)
        value=json.loads(output.getvalue());self.assertFalse(value['installed']['verified'])
        self.assertFalse(value['runtime']['verified']);self.assertEqual(value['reason'],'verification_incomplete')

    def test_worker_deadline_descriptor_isolation_reap_and_no_fd_growth(self):
        fd=os.open(self.root/'private',os.O_CREAT|os.O_RDWR,0o600)
        self.addCleanup(os.close,fd)
        before=set(os.listdir('/proc/self/fd'))
        with patch.dict(os.environ,WAYLAND_SOCKET=str(fd),PANEL_NOTIFICATIONS_FD=str(fd)):
            result=bounded(lambda:dict(env='WAYLAND_SOCKET' in os.environ,fd=os.path.exists('/proc/self/fd/'+str(fd))),1)
        self.assertEqual(result,dict(env=False,fd=False))
        for _ in range(3):
            worker=Worker(lambda:time.sleep(20));start=time.monotonic()
            with self.assertRaises(TimeoutError):worker.receive(start+.08)
            worker.close();self.assertLess(time.monotonic()-start,.5)
            with self.assertRaises(ChildProcessError):os.waitpid(worker.pid,os.WNOHANG)
        self.assertEqual(set(os.listdir('/proc/self/fd')),before)
    def test_worker_cancellation_reaps_probe_not_only_outer_process(self):
        receipt=self.root/'pid'
        def probe():
            subprocess.run(['/usr/bin/python3','-c',"import os,time,pathlib;pathlib.Path("+repr(str(receipt))+").write_text(str(os.getpid()));time.sleep(20)"],check=True,timeout=20)
        worker=Worker(probe)
        try:
            deadline=time.monotonic()+2
            while not receipt.exists() and time.monotonic()<deadline:time.sleep(.01)
            self.assertTrue(receipt.exists());pid=int(receipt.read_text());worker.close()
            self.assertFalse(Path('/proc',str(pid)).exists(),'probe must be reaped, not left an orphan/zombie')
        finally:worker.close()

if __name__=='__main__':unittest.main()

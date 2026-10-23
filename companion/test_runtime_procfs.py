"""Real procfs/ELF regression tests; synthetic processes are never native evidence."""
import copy
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time
import unittest

from bundle_contract import cohort_id, classify_compatibility, runtime_id
from runtime_identity import _process, running_identity
from test_generation_contract import catalog_fixture


class Procfs(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(); self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name); self.proc = self.root/'proc'; self.proc.mkdir()
        self.children = []; self.addCleanup(self.stop)
        self.catalog = catalog_fixture(); self.row = self.catalog['cohorts'][0]
        self.profile = self.row['runtime']; self.profile['processes'] = {}
        for name in ('cosmic-comp', 'cosmic-session', 'cosmic-settings-daemon', 'cosmic-applet-audio'):
            binary = self.root/name; shutil.copy2('/usr/bin/sleep', binary)
            child = self.start(name)
            measured = _process(Path('/proc')/str(child.pid))
            measured.pop('start'); measured['count'] = 1
            self.profile['processes'][name] = measured
        self.bind(self.row)
        self.manifest = dict(format='halcyon-islands-bundle',version=2,verified=True,payload_sha256='a'*64)

    def start(self, name):
        child = subprocess.Popen([str(self.root/name), '90'], env={**os.environ,'XDG_SESSION_ID':'procfs-test'})
        self.children.append(child)
        (self.proc/str(child.pid)).symlink_to(Path('/proc')/str(child.pid),target_is_directory=True)
        deadline = time.monotonic()+2
        while time.monotonic() < deadline:
            try:
                if os.readlink(Path('/proc')/str(child.pid)/'exe') == str(self.root/name):
                    _process(Path('/proc')/str(child.pid))
                    return child
            except (OSError, ValueError):
                pass
            time.sleep(.001)
        self.fail('test process did not exec')

    def stop(self):
        for child in self.children:
            if child.poll() is None: child.terminate()
            child.wait()

    def bind(self, row):
        row['processes']={name:item['executable'] for name,item in row['runtime']['processes'].items()}
        row['id'] = cohort_id(row)
        row['runtime_compatibility'] = [dict(id=runtime_id(row['runtime']),evidence_id=row['evidence']['id'],reason='Explicit synthetic fixture equivalence, NOT native evidence')]

    def measure(self, catalog=None):
        return running_identity(catalog or self.catalog, self.proc, session='procfs-test')

    def classify(self, row=None, catalog=None):
        return classify_compatibility(dict(verified=True,cohort=(row or self.row)['id']),self.measure(catalog),self.manifest,catalog or self.catalog)

    def test_required_applets_and_truncated_comm_are_measured(self):
        self.assertEqual(self.classify()['state'], 'ready')
        self.assertNotIn('cohort', self.measure())
        child = self.children[-1]; child.terminate(); child.wait()
        self.assertFalse(self.measure()['verified'])
        self.assertEqual(self.classify()['state'], 'logout_required')

    def test_unrelated_zombie_exit_and_vanished_proc_do_not_invalidate(self):
        zombie = subprocess.Popen(['/usr/bin/true']); self.children.append(zombie)
        os.waitid(os.P_PID,zombie.pid,os.WEXITED|os.WNOWAIT)
        (self.proc/str(zombie.pid)).symlink_to(Path('/proc')/str(zombie.pid),target_is_directory=True)
        (self.proc/'99999999').symlink_to('/proc/99999999',target_is_directory=True)
        self.assertTrue(self.measure()['verified'])
        zombie.wait()
        self.assertTrue(self.measure()['verified'])

    def test_required_zombie_and_unreadable_candidate_fail_closed(self):
        child = self.children[-1]; child.terminate()
        os.waitid(os.P_PID,child.pid,os.WEXITED|os.WNOWAIT)
        self.assertFalse(self.measure()['verified'])
        child.wait()
        self.start('cosmic-applet-audio')
        self.assertTrue(self.measure()['verified'])
        # Candidate with a truncated required comm but inaccessible exe is unsafe.
        bad = self.proc/'99999998'; bad.mkdir(); (bad/'comm').write_text('cosmic-settings')
        self.assertFalse(self.measure()['verified'])

    def test_app_only_update_old_running_new_installed_and_mixed(self):
        new = copy.deepcopy(self.row)
        new['packages']['cosmic-applets'] = dict(version='2',sha256='e'*64,files={'/usr/bin/cosmic-applet-audio':'f'*64})
        # An update replaces an inode while the old process keeps executing it.
        binary = self.root/'cosmic-applet-audio'; replacement=self.root/'replacement'
        replacement.write_bytes(binary.read_bytes()+b'app-only rebuilt ELF');replacement.chmod(0o755)
        os.replace(replacement,binary)
        old = self.measure(); self.assertTrue(old['verified'])
        self.assertEqual(self.classify()['state'],'ready')
        new['runtime']['processes']['cosmic-applet-audio']['sha256'] = __import__('bundle_contract').digest(binary)
        new['runtime']['processes']['cosmic-applet-audio']['mapped'][str(binary)] = __import__('bundle_contract').digest(binary)
        self.bind(new); catalog={**self.catalog,'cohorts':[self.row,new]}
        self.assertEqual(self.classify(new,catalog)['reason'],'installed_running_differ')
        self.assertFalse(self.measure({**catalog,'cohorts':[new]})['verified'])
        self.children[-1].terminate();self.children[-1].wait();self.start('cosmic-applet-audio')
        self.assertEqual(self.classify(new,catalog)['state'],'ready')
        # A partial, unknown comp rebuild matches neither profile.
        binary=self.root/'cosmic-comp'; replacement.write_bytes(binary.read_bytes()+b'unknown mixed');replacement.chmod(0o755);os.replace(replacement,binary)
        self.children[0].terminate();self.children[0].wait();self.start('cosmic-comp')
        self.assertFalse(self.measure(catalog)['verified'])

    def test_unchanged_code_rebuild_needs_explicit_qualified_binding(self):
        rebuilt=copy.deepcopy(self.row);rebuilt['packages']['cosmic-comp']['version']='1.0+rebuild'
        self.bind(rebuilt);catalog={**self.catalog,'cohorts':[self.row,rebuilt]}
        self.assertEqual(self.classify(rebuilt,catalog)['state'],'ready')
        self.assertEqual(self.classify(rebuilt,{**catalog,'cohorts':[rebuilt]})['state'],'ready')
        rebuilt['runtime_compatibility']=[]
        self.assertEqual(self.classify(rebuilt,catalog)['state'],'invalid_bundle')

    def test_required_exit_during_snapshot_retries_are_bounded(self):
        from unittest.mock import patch
        import runtime_identity
        real=runtime_identity._snapshot
        calls=[]
        def exiting(*args):
            value=real(*args);calls.append(True)
            if len(calls)==1:
                self.children[-1].terminate();self.children[-1].wait()
            return value
        with patch.object(runtime_identity,'_snapshot',side_effect=exiting):
            self.assertFalse(self.measure()['verified'])
        self.assertEqual(len(calls),4)
        self.start('cosmic-applet-audio')
        self.assertTrue(self.measure()['verified'])

    def test_declared_instance_count_and_multiple_matching_profiles(self):
        self.start('cosmic-applet-audio')
        self.assertFalse(self.measure()['verified'])
        self.profile['processes']['cosmic-applet-audio']['count']=2;self.bind(self.row)
        self.assertTrue(self.measure()['verified'])
        (self.proc/'sys/kernel').mkdir(parents=True)
        (self.proc/'sys/kernel/osrelease').write_text('scoped-kernel')
        other=copy.deepcopy(self.row);other['packages']['cosmic-comp']['version']='1+scope'
        other['runtime']['constraints']={'kernel_release':'scoped-kernel'};self.bind(other)
        catalog={**self.catalog,'cohorts':[self.row,other]}
        self.assertEqual(len(self.measure(catalog)['profiles']),2)
        self.assertEqual(self.classify(other,catalog)['state'],'ready')

    def test_measurable_kernel_constraints_are_not_whole_kernel_hashes(self):
        (self.proc/'sys/kernel').mkdir(parents=True)
        (self.proc/'sys/kernel/osrelease').write_text('fixture-kernel')
        self.profile['constraints']={'kernel_release':'fixture-kernel'};self.bind(self.row)
        self.assertTrue(self.measure()['verified'])
        (self.proc/'sys/kernel/osrelease').write_text('different')
        self.assertFalse(self.measure()['verified'])

if __name__ == '__main__': unittest.main()

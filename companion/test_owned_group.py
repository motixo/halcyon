"""Owned-session group completion before fallback, with real child processes."""
import json
from pathlib import Path
import subprocess
import sys
import unittest
import os
import signal
import time
from unittest.mock import patch
import panel_launcher as launcher


class OwnedGroup(unittest.TestCase):
    def test_group_is_finished_before_handoff(self):
        for mode in ('leader-exit','outer-term','handoff','fallback','census-race'):
            with self.subTest(mode=mode):
                result=subprocess.run([sys.executable,'-B',str(Path(__file__).with_name('owned_group_fixture.py')),mode],
                                      text=True,capture_output=True,timeout=12)
                self.assertEqual(result.returncode,0,result.stdout+result.stderr)
                r=json.loads(result.stdout)
                self.assertEqual(r['live_at_return'],[],r)
                self.assertTrue(r['outsider_alive'],r)
                self.assertTrue(r['leader_reaped'],r)
                self.assertLess(r['elapsed'],3,r)
                self.assertEqual(set(r['harness_reaped_descendants']),{a['pid'] for a in r['actors']})
                if mode in ('handoff','fallback'):self.assertEqual(r['live_at_failed_marker'],[])
                if mode=='handoff':self.assertEqual(r['result'],75)
                if mode=='fallback':self.assertTrue(r['stock_started']);self.assertEqual(r['result'],0)
                if mode=='outer-term':self.assertTrue(r['result'][1])

class IdentityGuards(unittest.TestCase):
    def host(self):
        child=launcher.OwnedHost('/bin/true',(),None)
        self.addCleanup(child.stdout.close)
        deadline=time.monotonic()+1
        while child.poll() is None:
            self.assertLess(time.monotonic(),deadline);time.sleep(.005)
        return child

    def test_exited_leader_stays_waitable_until_group_finished(self):
        child=self.host()
        self.assertEqual(child.poll(),0)
        self.assertEqual(launcher.OwnedHost._stat(child.pid)[0],b'Z')
        self.assertEqual(os.waitid(os.P_PID,child.pid,os.WEXITED|os.WNOWAIT).si_pid,child.pid)
        self.assertEqual(child.finish(),0)
        with self.assertRaises(ChildProcessError):os.waitpid(child.pid,os.WNOHANG)
        with patch.object(launcher.os,'killpg') as kill:
            child.signal_group(signal.SIGKILL)
        kill.assert_not_called()

    def test_reused_identity_and_lost_parent_authority_never_signal(self):
        child=self.host(); original=child._stat(child.pid)
        try:
            for changed in ((original[0],original[1],original[2],original[3]+1),
                            (original[0],original[1]+1,original[2],original[3]),
                            (original[0],original[1],original[2]+1,original[3])):
                with self.subTest(identity=changed), patch.object(child,'_stat',return_value=changed), \
                     patch.object(launcher.os,'killpg') as kill:
                    with self.assertRaises(launcher.GroupCleanupError):child.signal_group(signal.SIGKILL)
                    kill.assert_not_called()
            with patch.object(launcher.os,'waitid',side_effect=ChildProcessError),patch.object(launcher.os,'killpg') as kill:
                with self.assertRaises(launcher.GroupCleanupError):child.signal_group(signal.SIGKILL)
                kill.assert_not_called()
        finally:child.finish()

    def test_incomplete_group_never_reports_handoff_success_and_is_bounded(self):
        child=self.host(); start=time.monotonic()
        with patch.object(launcher,'GROUP_CLEANUP_SECONDS',.06),patch.object(launcher,'GROUP_TERM_SECONDS',.01), \
             patch.object(child,'live_members',return_value=[(child.pid,child.identity[-1])]):
            with self.assertRaisesRegex(launcher.GroupCleanupError,'timeout'):child.finish()
        self.assertLess(time.monotonic()-start,.3)
        self.assertTrue(child.closed)
        with patch.object(launcher,'run_host',side_effect=launcher.GroupCleanupError('incomplete')), \
             patch.object(launcher,'stock_host') as stock:
            with self.assertRaises(launcher.GroupCleanupError):launcher.supervise('/host','/stock')
            stock.assert_not_called()

    def test_group_signal_failure_cannot_be_caught_as_spawn_failure(self):
        child=self.host()
        with patch.object(launcher.os,'killpg',side_effect=PermissionError):
            with self.assertRaisesRegex(launcher.GroupCleanupError,'signal_failed'):child.finish()
        self.assertTrue(child.closed)
        with self.assertRaises(ChildProcessError):os.waitpid(child.pid,os.WNOHANG)

    def test_setup_identity_error_is_not_misreported_as_not_started(self):
        start=time.monotonic()
        with patch.object(launcher.OwnedHost,'_stat',side_effect=PermissionError):
            with self.assertRaisesRegex(launcher.GroupCleanupError,'setup_failed'):
                launcher.OwnedHost('/bin/true',(),None)
        self.assertLess(time.monotonic()-start,.5)

    def test_real_proc_census_observes_exited_leader(self):
        child=self.host()
        try:
            # The real /proc census includes unrelated processes; only group
            # stat is needed, including an already exited/zombie leader.
            self.assertEqual(child.live_members(time.monotonic()+1),[])
        finally:child.finish()


if __name__=='__main__':unittest.main()

"""Issue #12: ordinary native applet discovery, separate from exact qualification."""
import json
import os
from pathlib import Path
import shutil
import unittest
from unittest.mock import patch
from panel_configuration import Snapshot
import test_panel_configuration as config_fixture
import test_practical_compatibility as runtime_fixture

IDS = ['com.system76.CosmicAppletWorkspaces',
       'net.tropicbliss.CosmicExtAppletCaffeine', 'org.example.AnotherApplet']

class Configured(config_fixture.Configuration):
    def configured(self):
        s=Snapshot(env=self.env);self.addCleanup(s.close);return s
    def add(self, name, command='sleep 90', extra=''):
        self.write(self.data/('applications/'+name+'.desktop'), '[Desktop Entry]\nExec='+command+'\n'+extra)
    def test_reported_and_arbitrary_ids_resolve_without_allowlist(self):
        for name in IDS:self.add(name)
        self.setbar('Panel',IDS)
        s=self.configured()
        self.assertEqual(s.counts,dict.fromkeys(IDS,1))
        self.assertEqual(set(s.applets),set(IDS))
        self.assertTrue(all('resolved' in a for a in s.applets.values()))
    def test_missing_and_bad_entries_do_not_reject_other_applets(self):
        self.add(IDS[0]);self.add(IDS[1],'/not-installed/applet')
        self.setbar('Panel',IDS)
        s=self.configured()
        self.assertNotIn('issue',s.applets[IDS[0]])
        self.assertEqual(s.applets[IDS[1]]['issue'],'applet_executable_missing')
        self.assertEqual(s.applets[IDS[2]]['issue'],'desktop_entry_missing')
        self.add(IDS[2],'"unterminated')
        self.assertTrue(s.changed())
        self.assertEqual(self.configured().applets[IDS[2]]['issue'],'desktop_entry_invalid')
    def test_empty_and_malformed_launchers_are_diagnostic(self):
        self.setbar('Panel',[IDS[0]])
        for command in ['""', '   ', '# comment', "applet 'unfinished"]:
            with self.subTest(command=command):
                self.add(IDS[0],command)
                self.assertIn(self.configured().applets[IDS[0]]['issue'],
                    ['desktop_exec_invalid','desktop_entry_invalid','desktop_exec_missing'])
    def test_native_first_entry_wins_even_when_broken(self):
        name=IDS[0];self.setbar('Panel',[name]);self.add(name,'/missing/applet')
        self.write(self.system/('applications/'+name+'.desktop'),'[Desktop Entry]\nExec=sleep 90\n')
        self.assertEqual(self.configured().applets[name]['issue'],'applet_executable_missing')
    def test_flatpak_and_environment_wrappers_are_not_rejected(self):
        self.setbar('Panel',IDS[:2]);self.add(IDS[0],'env EXAMPLE=1 sleep 90')
        self.add(IDS[1],'env flatpak run org.example.Caffeine', 'X-Flatpak=org.example.Caffeine\n')
        s=self.configured();self.assertTrue(all(a['wrapped'] for a in s.applets.values()))
        self.assertTrue(all('issue' not in a for a in s.applets.values()))
    def test_same_applet_multiple_times_and_displays_matches_native_occurrences(self):
        self.add(IDS[0]);self.setbar('Panel',[IDS[0],IDS[0]])
        with patch('panel_configuration.output_names',return_value=['DP-1','eDP-1']):
            self.assertEqual(self.configured().counts[IDS[0]],4)
    def test_executable_replacement_and_desktop_aba_are_observed(self):
        exe=self.root/'applet';shutil.copy2('/usr/bin/sleep',exe)
        self.add(IDS[0],str(exe)+' 90');self.setbar('Panel',[IDS[0]])
        s=self.configured();exe.rename(self.root/'old');shutil.copy2('/usr/bin/sleep',exe)
        self.assertTrue(s.changed())
        s=self.configured();entry=self.data/('applications/'+IDS[0]+'.desktop');raw=entry.read_bytes()
        entry.write_bytes(b'changed');entry.write_bytes(raw)
        self.assertTrue(s.changed())

class Startup(runtime_fixture.Processes):
    def test_reported_ids_reach_core_readiness_without_any_child(self):
        self.setbar('Panel',IDS)
        for n,name in enumerate(IDS):
            exe=self.bin/('custom-applet-'+str(n));shutil.copy2('/usr/bin/sleep',exe)
            self.write(self.data/('applications/'+name+'.desktop'),'[Desktop Entry]\nExec='+str(exe)+' 60\n')
        s=self.observation();host=self.host(0)
        self.assertTrue(s.inventory(host))
        self.assertEqual({x['applet'] for x in s.applet_issues(host)},set(IDS))
        self.assertTrue(all(x['reason']=='applet_not_observed' for x in s.applet_issues(host)))
    def test_missing_custom_entry_is_diagnostic_not_host_failure(self):
        self.setbar('Panel',[IDS[1]])
        s=self.observation();self.assertTrue(s.inventory(self.host(0)))
        self.assertEqual(s.applet_issues(),[dict(applet=IDS[1],reason='desktop_entry_missing')])
    def test_foreign_applet_cannot_satisfy_own_applet_observation(self):
        self.write(self.data/'applications/com.system76.CosmicAppletAudio.desktop','[Desktop Entry]\nExec=cosmic-applet-audio 60\n')
        # Match the actual executable path and argv, but keep the process foreign.
        import subprocess
        foreign=subprocess.Popen([str(self.bin/'cosmic-applet-audio'),'60'],env=self.env)
        self.actors.append(foreign)
        s=self.observation();host=self.host(0)
        self.assertTrue(s.inventory(host))
        self.assertEqual(s.applet_issues(host)[0]['reason'],'applet_not_observed')
    def test_owned_executable_and_arguments_distinguish_multiplex_applets(self):
        self.write(self.data/'applications/com.system76.CosmicAppletAudio.desktop','[Desktop Entry]\nExec=cosmic-applets 60\n')
        s=self.observation();host=self.host(1)
        self.assertTrue(s.inventory(host));self.assertEqual(s.applet_issues(host),[])

for cls,base in [(Configured,config_fixture.Configuration),(Startup,runtime_fixture.Processes)]:
    for name in dir(base):
        if name.startswith('test_') and name not in cls.__dict__:setattr(cls,name,None)

# The loop variables must not expose imported test classes to unittest.
del cls, base

if __name__=='__main__':unittest.main()

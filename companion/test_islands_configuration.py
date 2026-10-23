"""Patched source contract + real sidecar IO. No Rust/native execution."""
import copy
import hashlib
import json
from pathlib import Path
import unittest
from unittest.mock import patch

from bundle_contract import CONFIGURATION_SOURCE, CONFIGURATION_RESOLVER, verify_bundle, canonical, digest
from panel_configuration import islands_settings_enabled, UnsupportedSettings
import test_panel_configuration as base
from test_generation_contract import bundle_fixture

ARRANGEMENT='(together:(order:[Start,Center,End],position:5000),separate:(order:[Start,Center,End],positions:[0,5000,10000]))'
def settings(enabled=True):
    return f'(version:1,enabled:{str(enabled).lower()},placement:Together,gap:14,default_layout:{ARRANGEMENT},outputs:{{}})'

class IslandsConfiguration(unittest.TestCase):
    setUp=base.Configuration.setUp
    write=base.Configuration.write
    setbar=base.Configuration.setbar
    snapshot=base.Configuration.snapshot
    def sidecar(self,name='Panel'):
        return self.user/'io.github.desktopcards.DockIslands/v1'/name

    def test_enabled_disabled_absent_present_and_supported_names(self):
        for name in ('Panel','Dock','Other'):
            self.write(self.user/'com.system76.CosmicPanel/v1/entries',json.dumps([name]))
            for output in ('Name("DP-9")','Name("DP-1")','All','Active'):
                self.setbar(name,['Audio'],output)
                for enabled in (None,False,True):
                    path=self.sidecar(name)
                    if path.exists():path.unlink()
                    if enabled is not None:self.write(path,settings(enabled))
                    with self.subTest(name=name,output=output,enabled=enabled):
                        observed=self.snapshot()
                        expected=0 if output=='Name("DP-9")' and not(enabled and name in ('Dock','Panel')) else 1
                        self.assertEqual(observed.counts,{'audio':expected})
                        self.assertEqual(str(path) in observed.records,name in ('Dock','Panel'))
                        observed.close()

    def test_native_all_active_and_enabled_name_multiple_outputs(self):
        self.write(self.sidecar(),settings())
        with patch('panel_configuration.output_names',return_value=['DP-1','eDP-1']):
            for output,count in [('All',2),('Active',1),('Name("DP-9")',1),('Name("DP-1")',1)]:
                self.setbar('Panel',['Audio'],output)
                self.assertEqual(self.snapshot().counts,{'audio':count})

    def test_sidecar_changes_absence_creation_replacement_delete_and_aba(self):
        self.setbar('Panel',['Audio'],'Name("DP-9")')
        absent=self.snapshot();self.write(self.sidecar(),settings());self.assertTrue(absent.changed())
        enabled=self.snapshot();self.write(self.sidecar(),settings(False));self.assertTrue(enabled.changed())
        disabled=self.snapshot();self.assertNotEqual(enabled.identity,disabled.identity)
        self.assertNotEqual(absent.identity,enabled.identity)
        # Same bytes returning after a transient toggle remain invalidated.
        self.write(self.sidecar(),settings());aba=self.snapshot()
        self.write(self.sidecar(),settings(False));self.write(self.sidecar(),settings());self.assertTrue(aba.changed())
        replaced=self.snapshot();tmp=self.sidecar().with_name('replacement');self.write(tmp,settings());tmp.replace(self.sidecar());self.assertTrue(replaced.changed())
        deleted=self.snapshot();self.sidecar().unlink();self.assertTrue(deleted.changed())

    def test_validation_failures_are_disabled_without_hiding_native_name(self):
        good=settings()
        invalid=[
            'not valid RON', '(enabled:true)', good.replace('version:1','version:2'),
            good.replace('gap:14','gap:65'),good.replace('gap:14','gap:-1'),
            good.replace('enabled:true','enabled:1'),good.replace('placement:Together','placement:Unknown'),
            good.replace('position:5000','position:10001'),good.replace('positions:[0,5000,10000]','positions:[0,5000,10001]'),
            good.replace('[Start,Center,End]','[Start,Start,End]'),good.replace('[Start,Center,End]','[Start,End]'),
            good.replace('gap:14','gap:14,extra:1'),good.replace('gap:14','gap:14,gap:14'),
            good.replace('outputs:{}','outputs:{"":'+ARRANGEMENT+'}'),
            good.replace('outputs:{}','outputs:{"'+('é'*65)+'":'+ARRANGEMENT+'}'),
            good.replace('outputs:{}','outputs:{"bad\\nname":'+ARRANGEMENT+'}'),
            good.replace('outputs:{}','outputs:{'+','.join(json.dumps(str(n))+':'+ARRANGEMENT for n in range(33))+'}'),
            good.replace('outputs:{}','outputs:{"DP-1":'+ARRANGEMENT.replace('position:5000','position:10001')+'}'),
            good.replace('outputs:{}','outputs:()'), good+' trailing', 'x'*65537,
        ]
        for text in invalid:
            with self.subTest(text=text[:100]):
                self.write(self.sidecar(),text)
                for output,count in [('Name("DP-9")',0),('Name("DP-1")',1),('All',1),('Active',1)]:
                    self.setbar('Panel',['Audio'],output);self.assertEqual(self.snapshot().counts,{'audio':count})
        self.sidecar().write_bytes(b'\xff')
        self.setbar('Panel',['Audio'],'Name("DP-9")');self.assertEqual(self.snapshot().counts,{'audio':0})

    def test_valid_layouts_limits_and_unknown_ron_extensions(self):
        good=settings()
        variants=[good,good.replace('Together,gap','Separate,gap'),good.replace('gap:14','gap:64'),
                  good.replace('outputs:{}','outputs:{"'+('é'*64)+'":'+ARRANGEMENT+'}'),
                  good.replace('outputs:{}','outputs:{'+','.join(json.dumps(str(n))+':'+ARRANGEMENT for n in range(32))+'}'),
                  '/* comment */'+good.replace('gap:14','gap: /*nested /*ok*/ */ 14')]
        for text in variants:self.assertTrue(islands_settings_enabled(text))
        # Never interpret valid-but-unsupported RON syntax as disabled.
        for text in [good.replace('outputs:{}','outputs:{"\\u{44}P-1":'+ARRANGEMENT+'}'), 'Settings'+good,good.replace('gap:14','gap:0x0e')]:
            with self.assertRaises(UnsupportedSettings):islands_settings_enabled(text)

    def test_sidecar_never_uses_system_defaults_and_relative_root_refused(self):
        self.setbar('Panel',['Audio'],'Name("DP-9")')
        self.write(self.system/'cosmic/io.github.desktopcards.DockIslands/v1/Panel',settings())
        self.assertEqual(self.snapshot().counts,{'audio':0})
        self.env['XDG_CONFIG_HOME']=''
        with self.assertRaisesRegex(ValueError,'relative_config_root'):self.snapshot()

    def test_native_source_pins_and_bundle_reject_missing_or_wrong_source(self):
        root=Path(__file__).resolve().parents[1]
        patch_bytes=(root/'companion/patches/0001-dock-islands.patch').read_bytes()
        self.assertEqual(hashlib.sha256(patch_bytes).hexdigest(),CONFIGURATION_SOURCE['patch_sha256'])
        self.assertEqual(digest(root/'companion/dock-islands-core/src/lib.rs'),CONFIGURATION_SOURCE['core_sha256'])
        section=patch_bytes.split(b'+++ b/cosmic-panel-bin/src/islands.rs\n')[1].split(b'\n--- ')[0]+b'\n'
        islands=b''.join(line[1:] for line in section.splitlines(keepends=True) if line.startswith(b'+') and not line.startswith(b'+++'))
        self.assertEqual(hashlib.sha256(islands).hexdigest(),CONFIGURATION_SOURCE['islands_sha256'])
        library=self.root/'bundle';manifest,catalog=bundle_fixture(library)
        self.assertEqual(catalog['cohorts'][0]['runtime']['configuration']['resolver'],CONFIGURATION_RESOLVER)
        verify_bundle(library,'companion-manifest.json')
        for source in (None,{**CONFIGURATION_SOURCE,'islands_sha256':'0'*64}):
            bad=copy.deepcopy(catalog);bad['cohorts'][0]['runtime']['configuration']['native_source']=source
            (library/'catalog.json').write_bytes(canonical(bad))
            changed=copy.deepcopy(manifest);changed['files']['catalog.json']['sha256']=digest(library/'catalog.json')
            (library/'companion-manifest.json').write_bytes(canonical(changed))
            with self.assertRaisesRegex(ValueError,'configuration_native_source_mismatch'):verify_bundle(library,'companion-manifest.json')

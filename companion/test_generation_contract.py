"""Offline contract fixtures are not native qualification evidence."""
import copy
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
sys.path.insert(0, str(Path(__file__).parent))
from bundle_contract import *
import runtime_identity


def catalog_fixture(payload='a'*64):
    packages = {name: dict(version='1.0',sha256='b'*64,files={'/usr/bin/'+name:'c'*64})
                for name in ('cosmic-panel','cosmic-comp','cosmic-session','cosmic-settings-daemon')}
    row = dict(distribution='pop',suite='noble',architecture='amd64',packages=packages,
               processes={n:'/usr/bin/'+n for n in ('cosmic-comp','cosmic-session','cosmic-settings-daemon')},
               qualification='native-session',payload_sha256=payload,
               evidence=dict(id='synthetic-unit-fixture-NOT-native',image_sha256='d'*64,scope='complete-session'))
    row['id']=cohort_id(row)
    return dict(format=CATALOG,version=1,cohorts=[row])


def bundle_fixture(path, version='0.2.1', catalog=True):
    path.mkdir(parents=True,exist_ok=True)
    for name in HELPERS:
        (path/name).write_bytes(Path(__file__).with_name(name).read_bytes())
        (path/name).chmod(0o755 if name in ('install.py','panel_launcher.py','selector.py') else 0o644)
    (path/'cosmic-panel-islands').write_bytes(b'unit fixture, never execute');(path/'cosmic-panel-islands').chmod(0o755)
    files={p.name:dict(sha256=digest(p),mode=p.stat().st_mode&0o777) for p in path.iterdir() if p.name in (*HELPERS,'cosmic-panel-islands')}
    payload=identity(files)
    cat=catalog_fixture(payload) if catalog else dict(format=CATALOG,version=1,cohorts=[])
    (path/'catalog.json').write_bytes(canonical(cat));(path/'catalog.json').chmod(0o644)
    files['catalog.json']=dict(sha256=digest(path/'catalog.json'),mode=0o644)
    manifest=dict(format=FORMAT,version=2,protocol=1,upstream='c'*40,runtime_probe=1,package_version=version,
                  binary_sha256=files['cosmic-panel-islands']['sha256'],payload_sha256=payload,files=files)
    (path/'companion-manifest.json').write_bytes(canonical(manifest));(path/'companion-manifest.json').chmod(0o644)
    return manifest,cat


class Contract(unittest.TestCase):
    def setUp(self):
        self.catalog=catalog_fixture();self.cohort=self.catalog['cohorts'][0]['id']
        self.manifest=dict(format=FORMAT,version=2,verified=True,payload_sha256='a'*64)
        self.known=dict(verified=True,cohort=self.cohort)
    def classify(self, installed=None,running=None,manifest=None,catalog=None):
        return classify_compatibility(installed if installed is not None else self.known,
            running if running is not None else self.known,manifest or self.manifest,catalog or self.catalog)
    def test_admitted_identical_running_and_installed(self):
        self.assertEqual(self.classify()['state'],'ready')
    def test_unknown_running_never_ready(self):
        self.assertEqual(self.classify(running={})['state'],'logout_required')
    def test_old_running_new_installed_and_missing_mixed_packages(self):
        self.assertEqual(self.classify(running=dict(verified=True,cohort='0'*64))['reason'],'installed_running_differ')
        self.assertEqual(self.classify(installed={})['state'],'unsupported')
        changed=copy.deepcopy(self.catalog);changed['cohorts'][0]['packages']['cosmic-comp']['version']='new'
        self.assertEqual(self.classify(catalog=changed)['state'],'invalid_bundle')
    def test_manifest_catalog_elf_payload_binding(self):
        for change in ({'verified':False},{'version':1},{'payload_sha256':'e'*64}):
            self.assertNotEqual(self.classify(manifest={**self.manifest,**change})['state'],'ready')
        for level in ('development','source-only'):
            changed=copy.deepcopy(self.catalog);changed['cohorts'][0]['qualification']=level
            self.assertEqual(self.classify(catalog=changed)['state'],'unsupported')
    def test_every_component_tamper_mode_and_symlink(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);bundle_fixture(root)
            verify_bundle(root,'companion-manifest.json')
            for name in FILES:
                with self.subTest(name=name):
                    original=(root/name).read_bytes();(root/name).write_bytes(original+b'changed')
                    with self.assertRaises(ValueError):verify_bundle(root,'companion-manifest.json')
                    (root/name).write_bytes(original)
            (root/'selector.py').chmod(0o777)
            with self.assertRaises(ValueError):verify_bundle(root,'companion-manifest.json')
    def test_same_version_changed_installed_bytes_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);cat=catalog_fixture()
            for name,pkg in cat['cohorts'][0]['packages'].items():
                path=root/'usr/bin'/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(name.encode())
                pkg['files']['/usr/bin/'+name]=digest(path)
            cat['cohorts'][0]['id']=cohort_id(cat['cohorts'][0])
            versions={n:'1.0' for n in cat['cohorts'][0]['packages']}
            self.assertTrue(runtime_identity.installed_identity(cat,root,versions,dict(distribution='pop',suite='noble',architecture='amd64'))['verified'])
            (root/'usr/bin/cosmic-comp').write_bytes(b'same source suffix different binary')
            self.assertFalse(runtime_identity.installed_identity(cat,root,versions,dict(distribution='pop',suite='noble',architecture='amd64'))['verified'])
    def test_unknown_proc_never_ready(self):
        with tempfile.TemporaryDirectory() as tmp:
            self.assertFalse(runtime_identity.running_identity(self.catalog,Path(tmp),session='unit')['verified'])

if __name__=='__main__':unittest.main()

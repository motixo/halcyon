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

FIXTURE_BINARY_SHA = __import__('hashlib').sha256(b'unit fixture, never execute').hexdigest()
FIXTURE_REVIEW = b"SYNTHETIC REVIEW ONLY; NEVER NATIVE EVIDENCE"
FIXTURE_REVIEW_SHA = __import__("hashlib").sha256(FIXTURE_REVIEW).hexdigest()


def runtime_fixture(row):
    files = runtime_identity.inventory(row)
    profile = dict(version=4, processes={name: dict(executable=path, sha256=files[path],
                   mapped=files.copy(), count=1, lifecycle='session') for name, path in row['processes'].items()},
                   configuration=dict(native_source=CONFIGURATION_SOURCE,resolver='cosmic-panel-ca20ce3-islands-v2',max_outputs=1,applets={}),
                   constraints={}, unmeasured=['previously-read-resources', 'kernel-bytes', 'driver-firmware', 'anonymous-code'])
    profile['host_mapped'] = {}
    row['runtime'] = profile
    row['runtime_compatibility'] = [dict(id=runtime_id(profile), evidence_id=row['evidence']['id'],
                                       reason='Synthetic fixture explicit scope; not native qualification')]
    return profile


def bind_target(row, root=None):
    """Synthetic test authoring only. Never used by packaging/runtime/discovery."""
    from runtime_target import target_id
    profile = row['runtime']; profile['version']=4
    profile.setdefault('host_mapped', {})
    for name, process in profile['processes'].items():
        path='/usr/bin/'+name
        if name not in row['packages']: row['packages'][name]=dict(version='fixture-1',sha256='b'*64,files={})
        row['packages'][name]['files'][path]=process['sha256']
    libraries = {}
    for process in profile['processes'].values():
        for path, sha in process['mapped'].items():
            if path.startswith(('/usr/lib/', '/lib/','/lib64/')):
                libraries[path] = sha
    libraries.update(profile['host_mapped'])
    if libraries: row['packages']['fixture-libraries'] = dict(version='1',sha256='b'*64,files=libraries)
    for package in row['packages'].values():
        package.setdefault('architecture','amd64')
        package.setdefault('source',dict(identity='d'*64))
    row['id']=cohort_id(row)
    files={}
    for name, package in row['packages'].items():
        for path, sha in package['files'].items():
            actual = next((Path(p['executable']) for p in profile['processes'].values() if p['sha256']==sha), Path(path))
            if root is not None:
                destination=root/path.lstrip('/')
                if not destination.exists() and Path(path).is_file():
                    import shutil
                    destination.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(path,destination)
                actual=destination
            size=actual.stat().st_size if actual.is_file() else 0
            files[path]=dict(sha256=sha,size=size,package=name,archive_path=path,source_identity=package['source']['identity'],
                kind='library' if '/lib' in path else 'executable', resolution='root-contained',reason='Synthetic fixture NOT native qualification')
    target=dict(version=1,scope='islands-runtime',qualification_cohort_id=row['id'],payload_sha256=row['payload_sha256'],evidence_id=row['evidence']['id'],
        profile_id=runtime_id(profile),equivalence_id=identity(row['runtime_compatibility']),
        system={k:row[k] for k in ('distribution','suite','architecture')},
        packages={n:{k:p[k] for k in ('version','architecture','sha256')} for n,p in row['packages'].items()},installed_files=files,
        dependency_review=dict(method='elf-closure-and-source-review-v1',record_sha256=FIXTURE_REVIEW_SHA,roots=list(files),dynamic_providers=[],resources=[]),
        native_scope=dict(graphics='synthetic-NOT-native',review_sha256=FIXTURE_REVIEW_SHA,constraints_sha256=identity(profile['constraints'])),work=dict(files=len(files),bytes=sum(f['size'] for f in files.values())))
    target['id']=target_id(target);row['runtime_target']=target
    return target


def installed(row):
    return dict(format='halcyon-installed-runtime',version=1,scope='islands-runtime',verified=True,targets=[row['runtime_target']['id']])


def measured(row):
    return dict(format='halcyon-runtime-measurement',version=4,phase='running',verified=True,host=dict(pid=1,sha256=FIXTURE_BINARY_SHA),profiles=[runtime_id(row['runtime'])], configurations={runtime_id(row['runtime']):dict(sha256='e'*64,children={})})


def catalog_fixture(payload='a'*64):
    packages = {name: dict(version='1.0',sha256='b'*64,files={'/usr/bin/'+name:'c'*64})
                for name in ('cosmic-panel','cosmic-comp','cosmic-session','cosmic-settings-daemon','cosmic-notifications')}
    row = dict(distribution='pop',suite='noble',architecture='amd64',packages=packages,
               processes={n:'/usr/bin/'+n for n in ('cosmic-comp','cosmic-session','cosmic-settings-daemon','cosmic-notifications')},
               qualification='native-session',payload_sha256=payload,
               evidence=dict(id='synthetic-unit-fixture-NOT-native',image_sha256='d'*64,scope='complete-session'))
    runtime_fixture(row)
    bind_target(row)
    return dict(format=CATALOG,version=2,cohorts=[row])


def bundle_fixture(path, version='0.2.1', catalog=True):
    path.mkdir(parents=True,exist_ok=True)
    for name in HELPERS:
        (path/name).write_bytes(Path(__file__).with_name(name).read_bytes())
        (path/name).chmod(0o755 if name in ('install.py','panel_launcher.py','selector.py') else 0o644)
    (path/'cosmic-panel-islands').write_bytes(b'unit fixture, never execute');(path/'cosmic-panel-islands').chmod(0o755)
    files={p.name:dict(sha256=digest(p),mode=p.stat().st_mode&0o777) for p in path.iterdir() if p.name in (*HELPERS,'cosmic-panel-islands')}
    payload=identity(files)
    cat=catalog_fixture(payload) if catalog else dict(format=CATALOG,version=2,cohorts=[])
    (path/'catalog.json').write_bytes(canonical(cat));(path/'catalog.json').chmod(0o644)
    files['catalog.json']=dict(sha256=digest(path/'catalog.json'),mode=0o644)
    manifest=dict(format=FORMAT,version=2,protocol=1,upstream=CONFIGURATION_SOURCE['upstream'],runtime_probe=1,package_version=version,
                  binary_sha256=files['cosmic-panel-islands']['sha256'],payload_sha256=payload,files=files)
    (path/'companion-manifest.json').write_bytes(canonical(manifest));(path/'companion-manifest.json').chmod(0o644)
    return manifest,cat


class Contract(unittest.TestCase):
    def setUp(self):
        self.catalog=catalog_fixture();self.cohort=self.catalog['cohorts'][0]['id']
        self.manifest=dict(upstream=CONFIGURATION_SOURCE['upstream'],format=FORMAT,version=2,verified=True,payload_sha256='a'*64,binary_sha256=FIXTURE_BINARY_SHA)
        self.known=installed(self.catalog['cohorts'][0])
    def classify(self, installed=None,running=None,manifest=None,catalog=None):
        return classify_compatibility(installed if installed is not None else self.known,
            running if running is not None else measured(self.catalog['cohorts'][0]),manifest or self.manifest,catalog or self.catalog)
    def test_admitted_identical_running_and_installed(self):
        self.assertEqual(self.classify()['state'],'ready')
    def test_unknown_running_never_ready(self):
        self.assertEqual(self.classify(running={})['state'],'logout_required')
    def test_old_running_new_installed_and_missing_mixed_packages(self):
        self.assertEqual(self.classify(running=dict(format='halcyon-runtime-measurement',version=4,phase='running',verified=True,host=dict(pid=1,sha256=FIXTURE_BINARY_SHA),profiles=['0'*64]))['reason'],'installed_running_differ')
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
            runtime_fixture(cat['cohorts'][0]);bind_target(cat['cohorts'][0],root)
            versions={n:'1.0' for n in cat['cohorts'][0]['packages']}
            self.assertTrue(runtime_identity.installed_identity(cat,root,versions,dict(distribution='pop',suite='noble',architecture='amd64'))['verified'])
            (root/'usr/bin/cosmic-comp').write_bytes(b'same source suffix different binary')
            self.assertFalse(runtime_identity.installed_identity(cat,root,versions,dict(distribution='pop',suite='noble',architecture='amd64'))['verified'])
    def test_running_code_inventory_ignores_noncode_graphics_mappings(self):
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);proc=root/'proc';proc.mkdir();cat=catalog_fixture();files={}
            for index,name in enumerate(('cosmic-comp','cosmic-session','cosmic-settings-daemon','cosmic-notifications')):
                exe=root/name;exe.write_bytes(name.encode());files[str(exe)]=digest(exe)
                cat['cohorts'][0]['processes'][name]=str(exe)
                process=proc/str(100+index);process.mkdir();(process/'comm').write_text(name)
                (process/'exe').symlink_to(exe);(process/'environ').write_bytes(b'XDG_SESSION_ID=native-fixture\0')
                (process/'stat').write_bytes(b'100 (fixture) ' + b'0 '*20)
                st=exe.stat();device=f'{os.major(st.st_dev):02x}:{os.minor(st.st_dev):02x}'
                (process/'maps').write_text(f'1000-2000 r-xp 0000 {device} {st.st_ino} '+str(exe)+'\n2000-3000 rw-s 0000 00:00 2 /memfd:graphics-buffer (deleted)\n')
            config=root/'config/cosmic/com.system76.CosmicPanel/v1';config.mkdir(parents=True);(config/'entries').write_text('[]')
            with patch.object(runtime_identity,'inventory',return_value=files), patch.dict(os.environ,{'HOME':str(root),'XDG_CONFIG_HOME':str(root/'config'),'XDG_DATA_HOME':str(root/'data'),'XDG_DATA_DIRS':str(root/'system')}), patch('panel_configuration.output_names',return_value=['fixture-output']):
                runtime_fixture(cat['cohorts'][0]);bind_target(cat['cohorts'][0])
                self.assertTrue(runtime_identity.running_identity(cat,proc,session='native-fixture',phase='prelaunch')['verified'])
                # An executable mapping of an unknown/deleted binary still fails.
                with (proc/'100/maps').open('a') as stream:stream.write('4000-5000 r-xp 0000 00:00 3 /unknown-library (deleted)\n')
                self.assertFalse(runtime_identity.running_identity(cat,proc,session='native-fixture',phase='prelaunch')['verified'])

    def test_unknown_proc_never_ready(self):
        with tempfile.TemporaryDirectory() as tmp:
            self.assertFalse(runtime_identity.running_identity(self.catalog,Path(tmp),session='unit')['verified'])

if __name__=='__main__':unittest.main()

"""Catalog bounds and actual-discovery structural checks; never qualification.

HALCYON_ACTUAL_CANDIDATE points to the retained production candidate read-only.
No native process, package installation, network or host preferences are used.
"""
import copy
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'companion'))
import bundle_contract as b
import catalog_capacity as capacity
from test_generation_contract import bundle_fixture, catalog_fixture

ACTUAL = os.environ.get('HALCYON_ACTUAL_CANDIDATE')
ACTUAL_SHA256 = '3c38ab7d2f22605b374ab1962c92239554240cce6ddb6343fadd178318ac02e1'


def structural_catalog(candidate, payload='a'*64):
    # The actual cohort identity/source/file maps are never reduced or changed.
    row = copy.deepcopy(candidate['cohort'])
    row.update(id=candidate['id'], qualification='development', payload_sha256=payload,
               evidence=dict(id='STRUCTURAL-NOT-QUALIFIED', image_sha256='b'*64,
                             scope='complete-session'))
    profile = dict(version=3, processes={name: dict(executable=path,
        sha256=row['packages'][name]['files'][path], mapped={path:row['packages'][name]['files'][path]},
        count=1, lifecycle='session') for name,path in row['processes'].items()},
        configuration=dict(resolver=b.CONFIGURATION_RESOLVER, native_source=b.CONFIGURATION_SOURCE,
                           max_outputs=1, applets={}), constraints={},
        unmeasured=['previously-read-resources','kernel-bytes','driver-firmware','anonymous-code'])
    row['runtime'] = profile
    row['runtime_compatibility'] = [dict(id=b.runtime_id(profile), evidence_id=row['evidence']['id'],
        reason='STRUCTURAL ONLY; NOT QUALIFIED; no process measurement')]
    return dict(format=b.CATALOG, version=1, cohorts=[row])


def write_bundle(path, catalog):
    manifest, _ = bundle_fixture(path, version='0.2.5~dev.catalog-capacity.1', catalog=False)
    for row in catalog['cohorts']:
        row['payload_sha256'] = manifest['payload_sha256']
    (path/'catalog.json').write_bytes(b.canonical(catalog)+b'\n')
    manifest['files']['catalog.json']['sha256'] = b.digest(path/'catalog.json')
    (path/'companion-manifest.json').write_bytes(b.canonical(manifest)+b'\n')
    return manifest


class Reader(unittest.TestCase):
    def setUp(self):
        self.work = tempfile.TemporaryDirectory()
        self.addCleanup(self.work.cleanup)
        self.root = Path(self.work.name)
        self.path = self.root/'catalog.json'
        self.cat = catalog_fixture()
        self.data = b.canonical(self.cat)
        self.path.write_bytes(self.data)

    def test_catalog_specific_bound_and_ordinary_records(self):
        self.assertEqual(b.CATALOG_BYTES, 64*1024**2)
        self.assertEqual(capacity.CATALOG_BYTES, b.CATALOG_BYTES)
        self.assertEqual(b.RECORD_BYTES, 16*1024**2)
        self.path.write_bytes(self.data+b' '*b.RECORD_BYTES)
        self.assertEqual(b.read_catalog(self.path),self.cat)
        for name in ('manifest.json','selection.json','generation.json'):
            path=self.root/name; path.write_bytes(b'{}')
            with path.open('r+b') as f:f.truncate(b.RECORD_BYTES+1)
            with self.assertRaisesRegex(ValueError,'not_bounded_record'):b.read_json(path)
        with self.path.open('wb') as f:f.truncate(b.CATALOG_BYTES+1)
        with self.assertRaisesRegex(ValueError,'not_bounded_record'):b.read_catalog(self.path)
        report=capacity.catalog_capacity(self.path)
        self.assertFalse(report['within_reader']);self.assertFalse(report['schema_valid'])

    def test_exact_boundary_and_truncation(self):
        cap=len(self.data)
        with patch.object(b,'CATALOG_BYTES',cap):
            self.assertEqual(b.read_catalog(self.path),self.cat)
            self.path.write_bytes(self.data+b' ')
            with self.assertRaisesRegex(ValueError,'not_bounded_record'):b.read_catalog(self.path)
        self.path.write_bytes(self.data[:-1])
        with self.assertRaises(ValueError):b.read_catalog(self.path)

    def test_opened_read_bound_even_when_stat_underreports(self):
        actual_fstat=os.fstat
        calls=[]
        class Stream:
            def __init__(self,fd,*args):self.real=original(fd,*args)
            def __enter__(self):return self
            def __exit__(self,*args):return self.real.__exit__(*args)
            def fileno(self):return self.real.fileno()
            def read(self,n):calls.append(n);return self.real.read(n)
        original=os.fdopen
        def underreported(fd):
            fields=list(actual_fstat(fd));fields[6]=1
            return os.stat_result(fields)
        with patch.object(b,'CATALOG_BYTES',32),patch.object(b.os,'fstat',underreported),patch.object(b.os,'fdopen',Stream):
            with self.assertRaisesRegex(ValueError,'record_changed_during_read'):b.read_catalog(self.path)
        self.assertEqual(calls,[2]) # opened size + sentinel, always <= cap + 1

    def test_replaced_truncated_and_same_size_rewrite_during_read(self):
        original=os.fdopen
        for kind in ('replace','truncate','rewrite'):
            with self.subTest(kind=kind):
                self.path.write_bytes(self.data)
                outer=self
                class Stream:
                    def __init__(self,fd,*args):self.real=original(fd,*args)
                    def __enter__(self):return self
                    def __exit__(self,*args):return self.real.__exit__(*args)
                    def fileno(self):return self.real.fileno()
                    def read(self,n):
                        data=self.real.read(n);before=outer.path.stat()
                        if kind=='replace':
                            other=outer.root/'replacement';other.write_bytes(data);other.replace(outer.path)
                        elif kind=='truncate':outer.path.write_bytes(data[:-1])
                        else:
                            outer.path.write_bytes(data[:-1]+b' ')
                            os.utime(outer.path,ns=(before.st_atime_ns,before.st_mtime_ns))
                        return data
                with patch.object(b.os,'fdopen',Stream):
                    with self.assertRaisesRegex(ValueError,'record_changed_during_read'):b.read_catalog(self.path)

    def test_nonregular_missing_and_symlink_paths(self):
        fifo=self.root/'fifo';os.mkfifo(fifo)
        descriptors=len(list(Path('/proc/self/fd').iterdir()))
        for path in (fifo,self.root):
            with self.subTest(path=path),self.assertRaises((OSError,ValueError)):b.read_catalog(path)
        self.assertEqual(len(list(Path('/proc/self/fd').iterdir())),descriptors)
        link=self.root/'link';link.symlink_to(self.path)
        with self.assertRaises(ValueError):b.read_catalog(link)
        folder=self.root/'folder';folder.symlink_to(self.root,target_is_directory=True)
        with self.assertRaises(ValueError):b.read_catalog(folder/'catalog.json')
        with self.assertRaises(FileNotFoundError):b.read_catalog(self.root/'missing')

    def test_json_schema_identity_and_digest_binding(self):
        for value in ([],None,True,'catalog'):
            with self.assertRaisesRegex(ValueError,'catalog_schema'):b.valid_catalog(value)
        for data in (b'[]',b'{',b'{}'):
            self.path.write_bytes(data)
            with self.assertRaises(ValueError):b.read_catalog(self.path)
        self.cat['cohorts'][0]['id']='f'*64;self.path.write_bytes(b.canonical(self.cat))
        with self.assertRaisesRegex(ValueError,'catalog_cohort_identity'):b.read_catalog(self.path)
        self.path.write_bytes(self.data)
        with self.assertRaisesRegex(ValueError,'bundle_component_mismatch'):b.read_catalog(self.path,'0'*64)
        with patch.object(Path,'read_bytes',side_effect=AssertionError('unbounded read')):
            self.assertEqual(b.read_catalog(self.path,hashlib.sha256(self.data).hexdigest())['format'],b.CATALOG)

    def test_bundle_binds_exact_parsed_catalog_and_rechecks_changes(self):
        library=self.root/'bundle';manifest,_=bundle_fixture(library)
        b.verify_bundle(library,'companion-manifest.json')
        path=library/'catalog.json';data=path.read_bytes();path.write_bytes(data+b' ')
        with self.assertRaisesRegex(ValueError,'bundle_component_mismatch'):b.verify_bundle(library,'companion-manifest.json')
        # Even valid JSON/schema at the same path is not cached or re-admitted.
        path.write_bytes(b.canonical(dict(format=b.CATALOG,version=1,cohorts=[])))
        with self.assertRaisesRegex(ValueError,'bundle_component_mismatch'):b.verify_bundle(library,'companion-manifest.json')

    def test_posix_path_rule_matches_original_including_traversal(self):
        for p in ('/usr/../x','/lib/a/../../x','/usr//./a','/usr/..hidden','/lib/a/..','/usr/a\\..','//usr/a','/usr/./../a'):
            self.assertEqual('..' in p.split('/'),'..' in Path(p).parts)
        for p in ('/usr/../x','/lib/a/../../x','/lib/a/..'):
            cat=catalog_fixture();row=cat['cohorts'][0]
            row['packages']['cosmic-panel']['files']={p:'a'*64};row['id']=b.cohort_id(row)
            with self.assertRaisesRegex(ValueError,'catalog_file_digest'):b.valid_catalog(cat)

    def test_selector_defers_catalog_hash_to_bounded_verified_reader(self):
        import selector, refresh
        home=self.root/'home';home.mkdir();library=self.root/'bundle'
        manifest,_=bundle_fixture(library)
        path=library/'catalog.json'
        with path.open('wb') as f:f.truncate(b.CATALOG_BYTES+1)
        manifest['files']['catalog.json']['sha256']=b.digest(path)
        raw=b.canonical(manifest);generation=hashlib.sha256(raw).hexdigest()
        (library/'manifest.json').write_bytes(raw);(library/'manifest.json').chmod(0o644)
        target=home/refresh.DATA/'companion/generations'/generation
        target.parent.mkdir(parents=True);library.rename(target)
        (home/refresh.POINTER).write_bytes(b.canonical(dict(generation=generation)))
        original=hashlib.file_digest
        def no_catalog_prehash(stream,*args):
            self.assertNotEqual(Path(stream.name).name,'catalog.json')
            return original(stream,*args)
        with patch.object(Path,'home',return_value=home),patch.object(selector,'fallback',return_value=77) as fallback, \
             patch.object(hashlib,'file_digest',no_catalog_prehash), \
             patch.object(refresh,'stage_installed_update',side_effect=AssertionError('must not stage')):
            self.assertEqual(selector.main(),77);fallback.assert_called_once()

    def test_staging_rechecks_bounded_catalog_bytes_before_copy(self):
        import refresh
        for growth in (True,False):
            with self.subTest(growth=growth):
                home=self.root/('home-'+str(growth));home.mkdir()
                library=self.root/('lib-'+str(growth));_,cat=bundle_fixture(library)
                (home/'installed-identity.json').write_bytes(b.canonical(dict(verified=True,cohort=cat['cohorts'][0]['id'])))
                original=refresh.read_catalog_bytes
                def replace(path,expected):
                    if growth:
                        with path.open('wb') as f:f.truncate(b.CATALOG_BYTES+1)
                    else:path.write_bytes(b.canonical(dict(format=b.CATALOG,version=1,cohorts=[])))
                    return original(path,expected)
                with patch.object(refresh,'read_catalog_bytes',replace):
                    with self.assertRaisesRegex(ValueError,'not_bounded_record|bundle_component_mismatch'):
                        refresh.stage_locked(home,library,session='fixture',fixture=True,enable=True)
                self.assertFalse((home/refresh.POINTER).exists())
                self.assertFalse((home/refresh.SELECTOR).exists())


@unittest.skipUnless(ACTUAL,'Set HALCYON_ACTUAL_CANDIDATE for retained actual-discovery checks')
class ActualCatalog(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        raw=Path(ACTUAL).read_bytes()
        if hashlib.sha256(raw).hexdigest()!=ACTUAL_SHA256:raise ValueError('actual_candidate_changed')
        cls.candidate=json.loads(raw)

    def setUp(self):
        self.work=tempfile.TemporaryDirectory();self.addCleanup(self.work.cleanup)
        self.root=Path(self.work.name)

    def test_full_cohort_bound_identity_and_unknown_never_ready(self):
        cat=structural_catalog(self.candidate);library=self.root/'bundle';manifest=write_bundle(library,cat)
        self.assertGreater((library/'catalog.json').stat().st_size,b.RECORD_BYTES)
        manifest,loaded=b.verify_bundle(library,'companion-manifest.json')
        row=loaded['cohorts'][0]
        self.assertEqual(row['id'],self.candidate['id'])
        self.assertEqual(row['packages'],self.candidate['cohort']['packages'])
        self.assertEqual(row['processes'],self.candidate['cohort']['processes'])
        self.assertEqual(sum(len(p['files']) for p in row['packages'].values()),165721)
        for p in row['packages'].values():
            for name in p['files']:self.assertEqual('..' in name.split('/'),'..' in Path(name).parts)
        self.assertEqual(b.classify_compatibility(dict(verified=True,cohort=row['id']),{},
            {**manifest,'verified':True},loaded)['state'],'unsupported')
        b.valid_runtime(row['runtime']) # structural schema only, never process evidence
        report=capacity.catalog_capacity(library/'catalog.json',self.candidate)
        self.assertTrue(report['within_reader'] and report['schema_valid'] and report['candidate_bound'])
        self.assertEqual(json.loads(Path(ACTUAL).read_bytes()),self.candidate)

    def test_two_full_rows_fit_but_no_third_cohort_assumption(self):
        cat=structural_catalog(self.candidate)
        second=copy.deepcopy(cat['cohorts'][0])
        # Clearly fictional performance row, not another authenticated distro.
        second['distribution']='structural-NOT-QUALIFIED';second['id']=b.cohort_id(second)
        cat['cohorts'].append(second)
        path=self.root/'two-rows.json';path.write_bytes(b.canonical(cat)+b'\n')
        self.assertLess(path.stat().st_size,b.CATALOG_BYTES)
        self.assertEqual(len(b.read_catalog(path)['cohorts']),2)
        self.assertGreater(path.stat().st_size*3//2,b.CATALOG_BYTES)

    def test_packaging_and_qualification_use_production_catalog_reader(self):
        spec=importlib.util.spec_from_file_location('islands_packager',ROOT/'packaging/islands/build-deb.py')
        packager=importlib.util.module_from_spec(spec);spec.loader.exec_module(packager)
        binary=self.root/'never-execute';binary.write_bytes(b'#!/bin/sh\nexit 77\n');binary.chmod(0o755)
        initial=dict(protocol=1,upstream=b.CONFIGURATION_SOURCE['upstream'],binary_sha256=b.digest(binary))
        manifest=self.root/'initial.json';manifest.write_bytes(b.canonical(initial))
        payload={name:dict(sha256=b.digest(ROOT/'companion'/name),mode=0o755 if name in ('install.py','panel_launcher.py','selector.py') else 0o644) for name in b.HELPERS}
        payload['cosmic-panel-islands']=dict(sha256=b.digest(binary),mode=0o755)
        cat=structural_catalog(self.candidate,b.identity(payload));path=self.root/'input-catalog.json';path.write_bytes(b.canonical(cat)+b'\n')
        package=packager.build(binary,manifest,'0.2.5~dev.catalog-capacity.1',self.root/'out',catalog=path)
        extract=self.root/'extract';subprocess.run(['dpkg-deb','-x',str(package),str(extract)],check=True)
        library=extract/'usr/lib/cosmic-ext-halcyon-islands'
        m,loaded=b.verify_bundle(library,'companion-manifest.json')
        self.assertEqual((library/'catalog.json').read_bytes(),path.read_bytes())
        self.assertEqual(loaded['cohorts'][0]['packages'],self.candidate['cohort']['packages'])
        import qualify
        image=self.root/'image-NOT-QUALIFIED.json'
        image.write_bytes(b.canonical(dict(format='halcyon-native-image',version=1,kind='complete-login-session',
            sha256='b'*64,recipe_sha256='c'*64,retained_inputs_sha256='d'*64,
            cohort_id=self.candidate['id'],graphics='STRUCTURAL ONLY; NOT QUALIFIED')))
        result=qualify.run(Path(ACTUAL),image,package,self.root/'blocked-qualification')
        self.assertEqual(result['state'],'blocked')
        self.assertIn('native_driver_required',result['reasons'])
        self.assertIn('reviewed_catalog_target_required',result['reasons'])
        self.assertEqual(result['catalog_sha256'],hashlib.sha256(path.read_bytes()).hexdigest())
        with path.open('wb') as f:f.truncate(b.CATALOG_BYTES+1)
        with self.assertRaisesRegex(ValueError,'not_bounded_record'):
            packager.build(binary,manifest,'0.2.5~dev.catalog-capacity.2',self.root/'oversize',catalog=path)


if __name__=='__main__':unittest.main()

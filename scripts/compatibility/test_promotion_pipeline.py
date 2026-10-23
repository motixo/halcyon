"""Synthetic end-to-end run/promotion/release tests, never native qualification."""
import copy
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

import qualify
import release_gate
from bundle_contract import canonical,digest,identity,verify_bundle
from test_generation_contract import catalog_fixture

ROOT=Path(__file__).resolve().parents[2]


class Pipeline(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup);self.root=Path(self.tmp.name)
        spec=importlib.util.spec_from_file_location('islands_builder',ROOT/'packaging/islands/build-deb.py')
        builder=importlib.util.module_from_spec(spec);spec.loader.exec_module(builder)
        binary=self.root/'binary';binary.write_text('#!/bin/sh\nexit 99\n')
        manifest=self.root/'manifest.json';manifest.write_bytes(canonical(dict(protocol=1,upstream='c'*40,binary_sha256=digest(binary))))
        with patch.dict(os.environ, SOURCE_DATE_EPOCH='1792627200'):
            pre=builder.build(binary,manifest,'0.2.1~test',self.root/'pre')
            subprocess.run(['dpkg-deb','-x',str(pre),str(self.root/'extract')],check=True,stdout=subprocess.DEVNULL)
            bundle=self.root/'extract/usr/lib/cosmic-ext-halcyon-islands'
            m,_=verify_bundle(bundle,'companion-manifest.json')
            catalog=catalog_fixture(m['payload_sha256']);row=catalog['cohorts'][0]
            cat=self.root/'catalog.json';cat.write_bytes(canonical(catalog))
            built=builder.build(binary,manifest,'0.2.1~test',self.root/'final',catalog=cat)
        self.package=self.root/'islands.deb';shutil.copy2(built,self.package)
        self.main=self.root/'main.deb'
        main_tree=self.root/'main-tree';(main_tree/'DEBIAN').mkdir(parents=True);(main_tree/'usr/bin').mkdir(parents=True)
        (main_tree/'DEBIAN/control').write_text('Package: cosmic-ext-halcyon\nVersion: 0.2.1~synthetic\nArchitecture: amd64\nMaintainer: Fixture <fixture@example.invalid>\nDescription: SYNTHETIC NOT NATIVE\n')
        (main_tree/'usr/bin/halcyon').write_bytes(b'SYNTHETIC MAIN EXECUTABLE NOT NATIVE')
        subprocess.run(['dpkg-deb','--root-owner-group','--build',str(main_tree),str(self.main)],check=True,stdout=subprocess.DEVNULL)
        self.original_main=self.main.read_bytes()
        (self.root/'source.tar').write_bytes(b'SYNTHETIC SOURCE NOT RELEASE')
        candidate=self.root/'candidate.json';candidate.write_bytes(canonical(dict(format='halcyon-cosmic-candidate',version=1,state='discovered',id=row['id'],cohort={k:row[k] for k in ('distribution','suite','architecture','packages','processes')})))
        image=self.root/'image.json';image.write_bytes(canonical(dict(format='halcyon-native-image',version=1,kind='complete-login-session',sha256='d'*64,recipe_sha256='e'*64,retained_inputs_sha256='f'*64,cohort_id=row['id'],graphics='SYNTHETIC ONLY')))
        driver=self.root/'synthetic-driver'
        driver.write_text('''#!/usr/bin/python3
import sys,json
from pathlib import Path
sys.dont_write_bytecode=True
sys.path[:0]=PATHS
from bundle_contract import canonical,identity,digest
from native_scenarios import SCENARIOS,SYSTEM_UPDATES
r=json.loads(Path(sys.argv[2]).read_bytes());out=Path(sys.argv[4]);artifacts={};scenarios={}
for name,kinds in {**SCENARIOS,**SYSTEM_UPDATES}.items():
 refs={}
 for kind in kinds:
  ref=name+'-'+kind+'.txt';(out/ref).write_text('SYNTHETIC TEST ONLY: NOT NATIVE EVIDENCE')
  artifacts[ref]={'sha256':digest(out/ref),'kind':kind};refs[kind]=[ref]
 scenarios[name]={'result':'pass','native':True,'evidence':refs,'assertions':[{'passed':True,'observed':'fixture','expected':'fixture'}]}
observed=dict(format='halcyon-native-observation',version=1,inputs=r['inputs'],request_sha256=identity(r),environment=dict(image_sha256=r['image']['sha256'],kind='complete-login-session',packages=r['candidate']['cohort']['packages'],graphics=r['image']['graphics'],dpkg_audit_clean=True,dependency_closure_verified=True),package_sha256=r['package_sha256'],bundle_sha256=r['bundle_sha256'],runtime_measurement=dict(format='halcyon-runtime-measurement',version=2,phase='running',verified=True,profiles=[identity(r['runtime_contract']['runtime'])]),offline=True,no_host_mounts=True,secrets_present=False,artifacts=artifacts,scenarios=scenarios)
observed['main']={'installed':r['main'],'running':{'executable':r['main']['executable'],'sha256':r['main']['executable_sha256'],'same_session':True,'evidence':['clean-login-processes.txt']}}
(out/'observations.json').write_bytes(canonical(observed))
'''.replace('PATHS',repr([str(ROOT/'companion'),str(ROOT/'scripts/compatibility')])));driver.chmod(0o755)
        self.evidence=self.root/'native-evidence'
        self.result=qualify.run(candidate,image,self.package,self.evidence,driver,self.main)
        self.assertEqual(self.result['state'],'passed',self.result)
        self.approval=self.root/'approval.json';self.approve()
        self.write_manifest()

    def approve(self):
        self.approval.write_bytes(canonical(dict(decision='approve',inputs=self.result['inputs'],package_sha256=digest(self.package),evidence_sha256=digest(self.evidence/'result.json'),review_reference='SYNTHETIC TEST: NOT AN APPROVAL')))

    def write_manifest(self):
        self.manifest=self.root/'release-manifest.json'
        self.manifest.write_bytes(canonical(dict(format='halcyon-release',version=1,release='0.2.1',system_updates_acceptance='passed',islands_package='islands.deb',artifacts={name:dict(sha256=digest(self.root/name),kind=kind) for name,kind in [('main.deb','main-deb'),('islands.deb','islands-deb'),('source.tar','corresponding-source')]})))

    def gate(self):
        return release_gate.verify(self.root,digest(self.manifest))

    def test_actual_run_promotion_release_reject_repack_outside_bundle(self):
        qualify.promotion(self.package,self.evidence,self.approval,self.main);self.gate()
        old=digest(self.package)
        subprocess.run(['dpkg-deb','-R',str(self.package),str(self.root/'repack')],check=True,stdout=subprocess.DEVNULL)
        (self.root/'repack/usr/share/doc/cosmic-ext-halcyon-islands/after-test').write_text('outside bundle mutation')
        subprocess.run(['dpkg-deb','--root-owner-group','--build',str(self.root/'repack'),str(self.package)],check=True,stdout=subprocess.DEVNULL)
        self.assertNotEqual(old,digest(self.package))
        self.result['package_sha256']=digest(self.package)
        (self.evidence/'result.json').write_bytes(canonical(self.result));self.approve();self.write_manifest()
        with self.assertRaises(ValueError):qualify.promotion(self.package,self.evidence,self.approval,self.main)
        with self.assertRaises(ValueError):self.gate()

    def test_every_authoritative_summary_identity_is_bound(self):
        original=copy.deepcopy(self.result)
        for key in ('package_sha256','bundle_sha256','payload_sha256','catalog_sha256','candidate_id','image_sha256','graphics','request_sha256','observation_sha256','inputs'):
            with self.subTest(key=key):
                self.result={**original,key:'changed'}
                (self.evidence/'result.json').write_bytes(canonical(self.result));self.approve()
                with self.assertRaises(ValueError):self.gate()

    def test_main_live_executable_identity_cannot_be_replaced_by_package_echo(self):
        path=self.evidence/'observations.json';original=json.loads(path.read_bytes())
        for field,value in [('sha256','0'*64),('same_session',False),('evidence',[])]:
            observed=copy.deepcopy(original);observed['main']['running'][field]=value
            path.write_bytes(canonical(observed));self.result['observation_sha256']=digest(path)
            (self.evidence/'result.json').write_bytes(canonical(self.result));self.approve()
            with self.assertRaises(ValueError):self.gate()

    def test_main_final_bytes_and_raw_system_update_scenarios_required(self):
        self.main.write_bytes(b'untested final main')
        self.write_manifest()
        with self.assertRaises(ValueError):self.gate()
        self.main.write_bytes(self.original_main);self.write_manifest()
        observed_path=self.evidence/'observations.json';observed=json.loads(observed_path.read_bytes())
        observed['scenarios'].pop('updates-apply-relogin');observed_path.write_bytes(canonical(observed))
        self.result['observation_sha256']=digest(observed_path)
        (self.evidence/'result.json').write_bytes(canonical(self.result));self.approve()
        with self.assertRaises(ValueError):self.gate()

if __name__=='__main__':unittest.main()

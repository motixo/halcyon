"""Offline public ELF graph fixture. Synthetic policy, never native admission."""
import copy
import json
from pathlib import Path
import shutil
import tempfile
import unittest
from author_runtime_target import audit,elf
from bundle_contract import canonical,digest,identity,runtime_id
from runtime_target import target_id
from test_generation_contract import bundle_fixture,runtime_fixture,bind_target

class Authoring(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup);self.root=Path(self.tmp.name)
        self.bundle=self.root/'bundle';self.manifest,self.catalog=bundle_fixture(self.bundle)
        self.sysroot=self.root/'public';self.row=self.catalog['cohorts'][0]
        for name,pkg in self.row['packages'].items():
            path='/usr/bin/'+name;dest=self.sysroot/path[1:];dest.parent.mkdir(parents=True,exist_ok=True)
            shutil.copy2('/usr/bin/true',dest);pkg['files'][path]=digest(dest)
        libs={}
        for name in ('libc.so.6','ld-linux-x86-64.so.2'):
            path='/usr/lib/x86_64-linux-gnu/'+name;dest=self.sysroot/path[1:];dest.parent.mkdir(parents=True,exist_ok=True)
            shutil.copy2(path,dest);libs[path]=digest(dest)
        self.row['packages']['explicit-library-fixture']=dict(version='1',sha256='e'*64,files=libs)
        shutil.copy2('/usr/bin/true',self.bundle/'cosmic-panel-islands')
        self.manifest['files']['cosmic-panel-islands']['sha256']=digest(self.bundle/'cosmic-panel-islands')
        self.manifest['binary_sha256']=self.manifest['files']['cosmic-panel-islands']['sha256']
        self.manifest['payload_sha256']=identity({k:v for k,v in self.manifest['files'].items() if k!='catalog.json'})
        self.row['payload_sha256']=self.manifest['payload_sha256']
        runtime_fixture(self.row);bind_target(self.row,self.sysroot)
        target=self.row['runtime_target'];files=target['installed_files'];by_name={Path(p).name:p for p in files}
        objects={}
        for path in [*files,'@payload']:
            actual=self.bundle/'cosmic-panel-islands' if path=='@payload' else self.sysroot/path[1:]
            observed=elf(actual);required=set(observed['needed'])|({observed['interpreter']} if observed['interpreter'] else set())
            objects[path]={**observed,'resolved':{n:by_name[Path(n).name] for n in required}}
        self.review=dict(format='halcyon-runtime-dependency-review',version=1,objects=objects,
            **{k:target['dependency_review'][k] for k in ('roots','dynamic_providers','resources')},
            source_review=['SYNTHETIC readelf fixture, not native scope/evidence'])
        self.native=dict(format='halcyon-runtime-scope-review',version=1,graphics='synthetic-NOT-native',constraints={},
            runtime_discriminators=['SYNTHETIC mapping fixture only'],source_review=['NOT QUALIFIED'])
        self.save()
    def save(self):
        self.dep=self.root/'dependency.json';self.dep.write_bytes(canonical(self.review))
        self.scope=self.root/'scope.json';self.scope.write_bytes(canonical(self.native))
        target=self.row['runtime_target'];target['dependency_review']['record_sha256']=digest(self.dep)
        target['native_scope']['review_sha256']=digest(self.scope);target['id']=target_id(target)
        (self.bundle/'catalog.json').write_bytes(canonical(self.catalog))
        self.manifest['files']['catalog.json']['sha256']=digest(self.bundle/'catalog.json')
        (self.bundle/'companion-manifest.json').write_bytes(canonical(self.manifest))
    def run_audit(self):return audit(self.catalog,self.row['id'],self.sysroot,self.bundle,self.dep,self.scope)
    def test_device_observation_required_and_renderer_is_only_description(self):
        from test_runtime_devices import make_device
        device=make_device(self.root/'sys')[2]
        self.row['runtime']['constraints']={'drm_devices':[device]}
        self.row['runtime_compatibility'][0]['id']=runtime_id(self.row['runtime'])
        self.row['runtime_target']['profile_id']=runtime_id(self.row['runtime'])
        self.row['runtime_target']['equivalence_id']=identity(self.row['runtime_compatibility'])
        self.row['runtime_target']['native_scope']['constraints_sha256']=identity(self.row['runtime']['constraints'])
        self.native['constraints']=self.row['runtime']['constraints'];self.save()
        with self.assertRaisesRegex(ValueError,'device_observation'):self.run_audit()
        self.native['device_observation']={'format':'halcyon-drm-device-observation','version':1,'devices':[device]}
        self.save()
        self.assertEqual(self.run_audit()['state'],'STRUCTURAL-NOT-QUALIFIED')
        self.native['device_observation']['devices']=[{**device,'device':'0x1234'}];self.save()
        with self.assertRaisesRegex(ValueError,'device_observation'):self.run_audit()

    def test_public_elf_graph_positive_and_missing_dependency_rejected(self):
        result=self.run_audit()
        self.assertEqual(result['state'],'STRUCTURAL-NOT-QUALIFIED')
        self.assertEqual(result['files'],len(self.row['runtime_target']['installed_files']))
        self.assertEqual(result['runtime_target_id'],self.row['runtime_target']['id'])
        self.review['objects']['@payload']['resolved'].clear();self.save()
        with self.assertRaisesRegex(ValueError,'unaccounted'):self.run_audit()
    def test_no_observed_mapping_trace_can_replace_source_review_or_exact_bytes(self):
        self.review['source_review']=[];self.save()
        with self.assertRaisesRegex(ValueError,'source_review'):self.run_audit()
        self.review['source_review']=['SYNTHETIC'];self.save()
        (self.sysroot/'usr/bin/cosmic-comp').write_bytes(b'changed')
        with self.assertRaises(ValueError):self.run_audit()

if __name__=='__main__':unittest.main()

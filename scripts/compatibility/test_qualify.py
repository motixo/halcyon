import copy
import json
from pathlib import Path
import sys
import tempfile
import unittest
sys.path.insert(0,str(Path(__file__).parent))
from qualify import validate_result, artifact_map, input_identities
from native_scenarios import SCENARIOS
from bundle_contract import identity,digest

class Qualification(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup);self.root=Path(self.tmp.name)
        self.request=dict(image=dict(sha256='a'*64,graphics='software-virtio-single'),candidate=dict(cohort=dict(packages={'cosmic-comp':'fixture'})),package_sha256='b'*64,bundle_sha256='c'*64)
        self.request.update(payload_sha256='d'*64,catalog_sha256='e'*64,evidence_id='synthetic',runtime_contract={'runtime':{'fixture':True}},scenarios=list(SCENARIOS))
        self.request['candidate']['id']='f'*64
        self.request['inputs']=input_identities(self.request)
        artifacts={};scenarios={}
        for name,kinds in SCENARIOS.items():
            evidence={}
            for kind in kinds:
                ref=name+'-'+kind+'.txt';(self.root/ref).write_text('synthetic unit test; never native evidence')
                artifacts[ref]=dict(sha256=digest(self.root/ref),kind=kind);evidence[kind]=[ref]
            scenarios[name]=dict(result='pass',native=True,evidence=evidence,assertions=[dict(passed=True,observed='fixture',expected='fixture')])
        self.observed=dict(inputs=self.request['inputs'],runtime_measurement=dict(format='halcyon-runtime-measurement',version=1,verified=True,profiles=[identity(self.request['runtime_contract']['runtime'])]),format='halcyon-native-observation',version=1,request_sha256=identity(self.request),
            environment=dict(image_sha256='a'*64,kind='complete-login-session',packages={'cosmic-comp':'fixture'},graphics='software-virtio-single',dpkg_audit_clean=True,dependency_closure_verified=True),
            package_sha256='b'*64,bundle_sha256='c'*64,offline=True,no_host_mounts=True,secrets_present=False,artifacts=artifacts,scenarios=scenarios)
    def test_schema_validation_is_not_native_execution(self):
        self.assertEqual(validate_result(self.request,self.observed,self.root),[])
    def test_socket_only_missing_bars_native_logout_notifications_fail(self):
        for name in SCENARIOS:
            with self.subTest(name=name):
                observed=copy.deepcopy(self.observed);observed['scenarios'][name]={'result':'pass','native':True,'evidence':{'ipc':['responsive']}}
                self.assertTrue(validate_result(self.request,observed,self.root))
    def test_wrong_guest_mixed_packages_graphics_and_artifact_fail(self):
        for key,value in [('packages',{}),('image_sha256','e'*64),('kind','nested-compositor'),('graphics','physical-gpu'),('dpkg_audit_clean',False)]:
            observed=copy.deepcopy(self.observed);observed['environment'][key]=value
            self.assertTrue(validate_result(self.request,observed,self.root))
        observed=copy.deepcopy(self.observed);observed['package_sha256']='e'*64
        self.assertIn('guest_artifact_mismatch',validate_result(self.request,observed,self.root))
    def test_no_post_evidence_mutation_or_path_escape(self):
        name=next(iter(self.observed['artifacts']));(self.root/name).write_text('changed')
        with self.assertRaises(ValueError):validate_result(self.request,self.observed,self.root)
        with self.assertRaises(ValueError):artifact_map(self.root,{'../outside':{'sha256':'e'*64}})

if __name__=='__main__':unittest.main()

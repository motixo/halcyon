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
from test_generation_contract import FIXTURE_REVIEW,FIXTURE_REVIEW_SHA

class Qualification(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup);self.root=Path(self.tmp.name)
        self.request=dict(image=dict(sha256='a'*64,graphics='software-virtio-single'),candidate=dict(cohort=dict(packages={'cosmic-comp':'fixture'})),package_sha256='b'*64,bundle_sha256='c'*64)
        self.request.update(companion_binary_sha256='b'*64,payload_sha256='d'*64,catalog_sha256='e'*64,evidence_id='synthetic',runtime_contract={'runtime':{'fixture':True,'processes':{}}, 'runtime_target':{'id':'a'*64,'dependency_review':{'record_sha256':FIXTURE_REVIEW_SHA},'native_scope':{'review_sha256':FIXTURE_REVIEW_SHA}}},scenarios=list(SCENARIOS))
        self.request['candidate']['id']='f'*64
        self.request['inputs']=input_identities(self.request)
        artifacts={};scenarios={}
        for name,kinds in SCENARIOS.items():
            evidence={}
            for kind in kinds:
                ref=name+'-'+kind+'.txt';(self.root/ref).write_text('synthetic unit test; never native evidence')
                artifacts[ref]=dict(sha256=digest(self.root/ref),kind=kind);evidence[kind]=[ref]
            scenarios[name]=dict(result='pass',native=True,evidence=evidence,assertions=[dict(passed=True,observed='fixture',expected='fixture')])
        (self.root/'review.txt').write_bytes(FIXTURE_REVIEW)
        artifacts['review.txt']=dict(sha256=FIXTURE_REVIEW_SHA,kind='runtime-review')
        self.observed=dict(runtime_reviews=dict(dependency='review.txt',native='review.txt'),installed_runtime=dict(format='halcyon-installed-runtime',version=1,scope='islands-runtime',verified=True,targets=[self.request['runtime_contract']['runtime_target']['id']]),inputs=self.request['inputs'],runtime_measurement=dict(format='halcyon-runtime-measurement',version=4,phase='running',verified=True,host=dict(pid=1,sha256=self.request['companion_binary_sha256']),profiles=[identity(self.request['runtime_contract']['runtime'])],configurations={identity(self.request['runtime_contract']['runtime']):dict(sha256='e'*64,children={})}),format='halcyon-native-observation',version=1,request_sha256=identity(self.request),
            environment=dict(image_sha256='a'*64,kind='complete-login-session',packages={'cosmic-comp':'fixture'},graphics='software-virtio-single',dpkg_audit_clean=True,dependency_closure_verified=True),
            package_sha256='b'*64,bundle_sha256='c'*64,offline=True,no_host_mounts=True,secrets_present=False,artifacts=artifacts,scenarios=scenarios)
    def test_missing_or_invalid_configuration_measurement_rejected(self):
        for observed in (None, [], {}, {'sha256':'e'*64,'children':{'unexpected':0}}, {'sha256':'invalid','children':{}}):
            raw=copy.deepcopy(self.observed)
            raw['runtime_measurement']['configurations']={identity(self.request['runtime_contract']['runtime']):observed}
            self.assertIn('runtime_measurement_mismatch',validate_result(self.request,raw,self.root))
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
    def test_prelaunch_eligibility_is_not_native_running_acceptance(self):
        observed=copy.deepcopy(self.observed)
        observed['runtime_measurement']['phase']='prelaunch'
        self.assertIn('runtime_measurement_mismatch',validate_result(self.request,observed,self.root))

    def test_no_post_evidence_mutation_or_path_escape(self):
        name=next(iter(self.observed['artifacts']));(self.root/name).write_text('changed')
        with self.assertRaises(ValueError):validate_result(self.request,self.observed,self.root)
        with self.assertRaises(ValueError):artifact_map(self.root,{'../outside':{'sha256':'e'*64}})

if __name__=='__main__':unittest.main()

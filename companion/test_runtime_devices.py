"""Synthetic sysfs IO only; no physical/native qualification is asserted."""
import copy
import os
from pathlib import Path
import tempfile
import time
import unittest
from bundle_contract import valid_runtime
from runtime_identity import _constraints
from test_generation_contract import catalog_fixture


def make_device(root, slot='0000:00:02.0', card=1, render=None):
    pci=root/'devices/pci0000:00'/slot;device=pci/'virtio0'
    device.mkdir(parents=True,exist_ok=True)
    for key,value in dict(vendor='0x1af4',device='0x1050',subsystem_vendor='0x1af4',subsystem_device='0x1100',**{'class':'0x030000'}).items():
        (pci/key).write_text(value+'\n')
    for node,driver,bus in [(pci,'virtio-pci','pci'),(device,'virtio_gpu','virtio')]:
        target=root/'bus'/bus/'drivers'/driver;target.mkdir(parents=True,exist_ok=True)
        (node/'driver').symlink_to(target)
    (device/'features').write_text('0'*64+'\n')
    drm=root/'class/drm';drm.mkdir(parents=True,exist_ok=True)
    for name in [f'card{card}']+([f'renderD{render}'] if render is not None else []):
        node=device/'drm'/name;node.mkdir(parents=True)
        (node/'device').symlink_to(device)
        (drm/name).symlink_to(node)
    expected=dict(vendor='0x1af4',device='0x1050',subsystem_vendor='0x1af4',subsystem_device='0x1100',pci_class='0x030000',drivers=['virtio_gpu','virtio-pci'],virtio_features='0'*64,primary_nodes=1,render_nodes=int(render is not None))
    return pci,device,expected


class Devices(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup)
        self.root=Path(self.tmp.name);self.sys=self.root/'sys';self.pci,self.device,self.expected=make_device(self.sys)
        self.profile=catalog_fixture()['cohorts'][0]['runtime'];self.profile['constraints']={'drm_devices':[self.expected]}
    def test_exact_bounded_device_inventory_is_an_admission_constraint(self):
        valid_runtime(self.profile)
        self.assertTrue(_constraints(self.profile['constraints'],self.root/'proc',self.sys))
        (self.pci/'device').write_text('0x1234\n')
        self.assertFalse(_constraints(self.profile['constraints'],self.root/'proc',self.sys))
    def test_numbers_are_not_hardware_identity_but_extra_device_is_not_ignored(self):
        from runtime_identity import drm_device_snapshot
        before=drm_device_snapshot(self.sys)[0]
        (self.sys/'class/drm/card1').rename(self.sys/'class/drm/card9')
        self.assertEqual(drm_device_snapshot(self.sys)[0],before)
        make_device(self.sys,slot='0000:00:03.0',card=2)
        self.assertFalse(_constraints(self.profile['constraints'],self.root/'proc',self.sys))
        self.assertTrue(_constraints({'drm_devices':[self.expected,self.expected]},self.root/'proc',self.sys))
    def test_missing_driver_unknown_bus_symlink_escape_and_oversize_refuse(self):
        from runtime_identity import drm_device_snapshot
        (self.device/'driver').unlink()
        with self.assertRaises((ValueError,OSError)):drm_device_snapshot(self.sys)
        outside=self.root/'outside';outside.mkdir();(self.device/'driver').symlink_to(outside)
        with self.assertRaises((ValueError,OSError)):drm_device_snapshot(self.sys)
        (self.device/'driver').unlink();(self.device/'driver').symlink_to(self.sys/'bus/virtio/drivers/virtio_gpu')
        (self.pci/'vendor').write_text('x'*4096)
        with self.assertRaises(ValueError):drm_device_snapshot(self.sys)
    def test_fifo_and_inventory_limit_fail_without_blocking(self):
        from runtime_identity import drm_device_snapshot
        (self.pci/'vendor').unlink();os.mkfifo(self.pci/'vendor');start=time.monotonic()
        with self.assertRaises(ValueError):drm_device_snapshot(self.sys)
        self.assertLess(time.monotonic()-start,.5)
        (self.pci/'vendor').unlink();(self.pci/'vendor').write_text('0x1af4\n')
        for n in range(257):(self.sys/'class/drm'/f'irrelevant{n}').touch()
        with self.assertRaises(ValueError):drm_device_snapshot(self.sys)
    def test_render_node_count_feature_change_and_kernel_requirements_preserved(self):
        (self.root/'proc/sys/kernel').mkdir(parents=True);(self.root/'proc/sys/kernel/osrelease').write_text('fixture\n')
        constraints={**self.profile['constraints'],'kernel_release':'fixture'}
        self.assertTrue(_constraints(constraints,self.root/'proc',self.sys))
        node=self.device/'drm/renderD128';node.mkdir();(node/'device').symlink_to(self.device)
        (self.sys/'class/drm/renderD128').symlink_to(node)
        self.assertFalse(_constraints(constraints,self.root/'proc',self.sys))
        constraints['drm_devices'][0]['render_nodes']=1
        self.assertTrue(_constraints(constraints,self.root/'proc',self.sys))
        (self.device/'features').write_text('1'+'0'*63+'\n')
        self.assertFalse(_constraints(constraints,self.root/'proc',self.sys))
        (self.device/'features').write_text('0'*64+'\n')
        (self.root/'proc/sys/kernel/osrelease').write_text('different\n')
        self.assertFalse(_constraints(constraints,self.root/'proc',self.sys))
    def test_documented_schema_binds_current_native_source(self):
        import json
        from bundle_contract import CONFIGURATION_SOURCE
        schema=json.loads((Path(__file__).parent/'compatibility/schema.json').read_text())
        runtime=schema['properties']['cohorts']['items']['properties']['runtime']['properties']
        self.assertEqual(runtime['configuration']['properties']['native_source']['const'],CONFIGURATION_SOURCE)
        self.assertEqual(set(runtime['constraints']['properties']['drm_devices']['items']['properties']),set(self.expected))

    def test_existing_module_constraints_remain_required(self):
        module=self.sys/'module/virtio_gpu';module.mkdir(parents=True)
        (module/'srcversion').write_text('fixture-module\n')
        constraints={**self.profile['constraints'],'module_versions':{'virtio_gpu':{'srcversion':'fixture-module'}}}
        self.assertTrue(_constraints(constraints,self.root/'proc',self.sys))
        (module/'srcversion').write_text('changed-module\n')
        self.assertFalse(_constraints(constraints,self.root/'proc',self.sys))
        (module/'srcversion').unlink()
        with self.assertRaises(OSError):_constraints(constraints,self.root/'proc',self.sys)

    def test_schema_rejects_descriptive_renderer_and_unbounded_or_ambiguous_values(self):
        for patch in [{'renderer':'llvmpipe'}, {'primary_nodes':True},{'vendor':'../../etc/shadow'},{'drivers':[]},{'virtio_features':'1'*1000}]:
            profile=copy.deepcopy(self.profile);profile['constraints']['drm_devices'][0].update(patch)
            with self.assertRaises(ValueError):valid_runtime(profile)
        for value in [[],[self.expected]*9]:
            profile=copy.deepcopy(self.profile);profile['constraints']['drm_devices']=value
            with self.assertRaises(ValueError):valid_runtime(profile)

if __name__=='__main__':unittest.main()

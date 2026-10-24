"""Synthetic sysfs IO only; no physical/native qualification is asserted."""
import copy
import json
import shutil
import os
from pathlib import Path
import tempfile
import time
import unittest
from bundle_contract import valid_runtime
from runtime_identity import _constraints
from test_generation_contract import catalog_fixture


CAPTURED=json.loads((Path(__file__).parent/'fixtures/drm-pci-parent.json').read_text())['node']
CAPTURED_BITS=CAPTURED['virtio_children']['virtio0']['features'].strip()


def make_device(root, slot='0000:00:02.0', card=1, render=None, topology='virtio', features='0'*64):
    pci=root/'devices/pci0000:00'/slot;device=pci/f'virtio{card-1}'
    device.mkdir(parents=True,exist_ok=True)
    for key,value in dict(vendor='0x1af4',device='0x1050',subsystem_vendor='0x1af4',subsystem_device='0x1100',**{'class':'0x030000'}).items():
        (pci/key).write_text(value+'\n')
    for node,driver,bus in [(pci,'virtio-pci','pci'),(device,'virtio_gpu','virtio')]:
        target=root/'bus'/bus/'drivers'/driver;target.mkdir(parents=True,exist_ok=True)
        (node/'driver').symlink_to(target)
        (node/'subsystem').symlink_to(root/'bus'/bus)
        aliases=root/'bus'/bus/'devices';aliases.mkdir(exist_ok=True)
        (aliases/node.name).symlink_to(node)
    for key,value in CAPTURED['virtio_children']['virtio0'].items():
        if key in ('vendor','device','status'):(device/key).write_text(value)
    (device/'features').write_text(features+'\n')
    owner=pci if topology=='pci' else device
    drm=root/'class/drm';drm.mkdir(parents=True,exist_ok=True)
    for name in [f'card{card}']+([f'renderD{render}'] if render is not None else []):
        node=owner/'drm'/name;node.mkdir(parents=True)
        (node/'device').symlink_to(owner)
        (drm/name).symlink_to(node)
    expected=dict(vendor='0x1af4',device='0x1050',subsystem_vendor='0x1af4',subsystem_device='0x1100',pci_class='0x030000',drivers=['virtio_gpu','virtio-pci'],virtio_features=features,primary_nodes=1,render_nodes=int(render is not None))
    return pci,device,expected


class Devices(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup)
        self.root=Path(self.tmp.name);self.sys=self.root/'sys';self.pci,self.device,self.expected=make_device(self.sys)
        self.profile=catalog_fixture()['cohorts'][0]['runtime'];self.profile['constraints']={'drm_devices':[self.expected]}
    def captured_pci(self):
        return make_device(self.root/'captured',render=128,topology='pci',features=CAPTURED_BITS)

    def test_captured_pci_parent_full_features_and_missing_driver_refuse(self):
        from runtime_identity import drm_device_snapshot
        pci,child,expected=self.captured_pci();root=self.root/'captured'
        self.assertEqual(len(CAPTURED_BITS),128)
        self.assertEqual(drm_device_snapshot(root)[0],[expected])
        changed=CAPTURED_BITS[:100]+('1' if CAPTURED_BITS[100]=='0' else '0')+CAPTURED_BITS[101:]
        (child/'features').write_text(changed+'\n')
        self.assertFalse(_constraints({'drm_devices':[expected]},self.root/'proc',root))
        (child/'features').write_text(CAPTURED_BITS+'\n');(child/'driver').unlink()
        with self.assertRaises((ValueError,OSError)):drm_device_snapshot(root)

    def test_virtio_vendor_device_are_not_pci_identity(self):
        from runtime_identity import drm_device_snapshot
        self.assertEqual((self.device/'device').read_text(),'0x0010\n')
        self.assertEqual(drm_device_snapshot(self.sys)[0],[self.expected])

    def test_ambiguous_child_wrong_bus_and_wrong_gpu_id_refuse(self):
        from runtime_identity import drm_device_snapshot
        pci,child,expected=self.captured_pci();root=self.root/'captured'
        second=pci/'virtio1';shutil.copytree(child,second,symlinks=True)
        with self.assertRaises(ValueError):drm_device_snapshot(root)
        shutil.rmtree(second)
        (child/'subsystem').unlink();(child/'subsystem').symlink_to(root/'bus/pci')
        with self.assertRaises((ValueError,OSError)):drm_device_snapshot(root)
        (child/'subsystem').unlink();(child/'subsystem').symlink_to(root/'bus/virtio')
        (child/'device').write_text('0x0001\n')
        with self.assertRaises(ValueError):drm_device_snapshot(root)

    def test_missing_child_bus_binding_link_aba_and_child_inventory_bound(self):
        from runtime_identity import drm_device_snapshot
        pci,child,expected=self.captured_pci();root=self.root/'captured'
        before=drm_device_snapshot(root)
        link=child/'driver';link.unlink();link.symlink_to(root/'bus/virtio/drivers/virtio_gpu')
        self.assertNotEqual(drm_device_snapshot(root),before)
        alias=root/'bus/virtio/devices'/child.name;alias.unlink()
        with self.assertRaises((ValueError,OSError)):drm_device_snapshot(root)
        alias.symlink_to(child)
        parked=self.root/'parked';child.rename(parked)
        with self.assertRaises((ValueError,OSError)):drm_device_snapshot(root)
        parked.rename(child)
        for n in range(257):(pci/f'unrelated{n}').touch()
        with self.assertRaisesRegex(ValueError,'inventory_limit'):drm_device_snapshot(root)

    def test_feature_width_is_finite_exact_and_no_null_transport_policy(self):
        from runtime_identity import drm_device_snapshot
        pci,child,expected=self.captured_pci();root=self.root/'captured'
        profile=copy.deepcopy(self.profile);profile['constraints']={'drm_devices':[expected]}
        valid_runtime(profile)
        for value in ['0'*63,'0'*65,'0'*127,'0'*129,'0'*257,'x'*128]:
            (child/'features').write_text(value+'\n')
            with self.assertRaises(ValueError):drm_device_snapshot(root)
            profile['constraints']['drm_devices'][0]['virtio_features']=value
            with self.assertRaises(ValueError):valid_runtime(profile)
        profile['constraints']['drm_devices'][0]['virtio_features']=None
        with self.assertRaises(ValueError):valid_runtime(profile)

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

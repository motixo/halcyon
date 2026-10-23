"""Pinned config IO semantics, synthetic compositor wire server; not native QA."""
import copy
import json
import os
from pathlib import Path
import socket
import struct
import tempfile
import threading
import unittest
from unittest.mock import patch
from panel_configuration import Snapshot, RESOLVER, ron, desktop, output_names

class Configuration(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup)
        self.root=Path(self.tmp.name);self.home=self.root/'home';self.home.mkdir()
        self.user=self.home/'.config/cosmic';self.data=self.home/'.local/share';self.system=self.root/'system'
        self.env={'HOME':str(self.home),'XDG_DATA_DIRS':str(self.system),'PATH':'/usr/bin:/bin'}
        self.policy=dict(resolver=RESOLVER,max_outputs=2,processes={'audio':'/usr/bin/sleep'},applets={'Audio':dict(process='audio',args=['90'])})
        self.setbar('Panel',['Audio']);self.write(self.user/'com.system76.CosmicPanel/v1/entries','["Panel"]')
        self.write(self.data/'applications/Audio.desktop','[Desktop Entry]\nName=Audio\nExec=sleep 90\n')
        self.patch=patch('panel_configuration.output_names',return_value=['DP-1']);self.patch.start();self.addCleanup(self.patch.stop)

    def write(self,path,text):path.parent.mkdir(parents=True,exist_ok=True);path.write_text(text)
    def setbar(self,name,plugins,output='All',root=None):
        base=(root or self.user)/('com.system76.CosmicPanel.'+name)/'v1'
        for key,value in dict(name=json.dumps(name),output=output,plugins_wings='None',plugins_center='Some('+json.dumps(plugins)+')').items():self.write(base/key,value)
    def snapshot(self):
        s=Snapshot(self.policy,self.env);self.addCleanup(s.close);return s
    def center(self):return self.user/'com.system76.CosmicPanel.Panel/v1/plugins_center'

    def test_same_policy_zero_one_two_across_bars_and_options(self):
        before=copy.deepcopy(self.policy)
        for count in range(3):
            self.setbar('Panel',['Audio'] if count else [])
            self.setbar('Dock',['Audio'])
            self.write(self.user/'com.system76.CosmicPanel/v1/entries',json.dumps(['Panel','Dock'] if count==2 else ['Panel']))
            self.assertEqual(self.snapshot().counts,{'audio':count})
        self.assertEqual(self.policy,before)
        self.write(self.center(),'None');self.write(self.user/'com.system76.CosmicPanel/v1/entries','["Panel"]')
        self.assertEqual(self.snapshot().counts,{'audio':0})

    def test_per_key_user_override_and_first_system_namespace(self):
        self.center().unlink()
        self.setbar('Panel',['Audio'],root=self.system/'cosmic')
        self.assertEqual(self.snapshot().counts,{'audio':1})
        self.write(self.center(),'Some([])');self.assertEqual(self.snapshot().counts,{'audio':0})
        self.center().unlink()
        # Existing higher-priority namespace (not per-file search) hides lower.
        (self.data/'cosmic/com.system76.CosmicPanel.Panel/v1').mkdir(parents=True)
        self.assertEqual(self.snapshot().counts,{'audio':0})

    def test_output_all_named_absent_and_active_counts(self):
        with patch('panel_configuration.output_names',return_value=['DP-1','DP-2']):
            self.assertEqual(self.snapshot().counts,{'audio':2})
            for output,count in [('Name("DP-1")',1),('Name("missing")',0),('Active',1)]:
                self.setbar('Panel',['Audio'],output);self.assertEqual(self.snapshot().counts,{'audio':count})
        self.policy['max_outputs']=1
        with patch('panel_configuration.output_names',return_value=['DP-1','DP-2']):
            with self.assertRaisesRegex(ValueError,'output_count'):self.snapshot()

    def test_wings_center_alias_executable_and_qualified_args(self):
        self.policy['applets']['Other']=dict(process='audio',args=['90'])
        self.write(self.data/'applications/Other.desktop','[Desktop Entry]\nExec="/usr/bin/sleep" "90"\n')
        self.write(self.center(),'Some(["Other",])')
        self.write(self.center().with_name('plugins_wings'),'Some((["Audio"], []))')
        self.assertEqual(self.snapshot().counts,{'audio':2})
        self.write(self.data/'applications/Other.desktop','[Desktop Entry]\nExec=/usr/bin/sleep 99\n')
        with self.assertRaisesRegex(ValueError,'arguments_unqualified'):self.snapshot()

    def test_desktop_precedence_nested_file_stem_and_malformed_override(self):
        target=self.data/'applications/Audio.desktop';target.unlink()
        self.write(self.system/'applications/nested/Audio.desktop','[Desktop Entry]\nExec=/usr/bin/sleep 90\n')
        self.assertEqual(self.snapshot().counts,{'audio':1})
        self.write(target,'[Desktop Entry]\nName=broken override\n')
        with self.assertRaisesRegex(ValueError,'exec_unsupported'):self.snapshot()

    def test_exec_path_shadow_and_unqualified_plugin_fail_closed(self):
        self.setbar('Panel',['Unknown'])
        with self.assertRaisesRegex(ValueError,'unqualified_configured'):self.snapshot()
        self.setbar('Panel',['Audio'])
        shadow=self.root/'bin/sleep';self.write(shadow,'not executed');shadow.chmod(0o755)
        self.env['PATH']=str(shadow.parent)+':/usr/bin'
        with self.assertRaisesRegex(ValueError,'exec_unqualified'):self.snapshot()

    def test_duplicate_plugin_unsupported_no_guess_and_malformed_ron(self):
        self.setbar('Panel',['Audio','Audio'])
        with self.assertRaisesRegex(ValueError,'duplicate_plugin'):self.snapshot()
        for text in ('Some(["Audio"]', 'Some([r"Audio"])','Some(["Au\\u0064io"])'):
            self.write(self.center(),text)
            with self.assertRaises(ValueError):self.snapshot()

    def test_config_and_desktop_aba_detected_and_output_change(self):
        for path in (self.center(),self.data/'applications/Audio.desktop'):
            snapshot=self.snapshot();old=path.read_bytes();path.write_bytes(old+b'\n');path.write_bytes(old)
            self.assertTrue(snapshot.changed())
        snapshot=self.snapshot()
        with patch('panel_configuration.output_names',return_value=['DP-2']):self.assertTrue(snapshot.changed())

    def test_higher_priority_absent_desktop_creation_detected(self):
        target=self.data/'applications/Audio.desktop';target.unlink()
        self.write(self.system/'applications/Audio.desktop','[Desktop Entry]\nExec=/usr/bin/sleep 90\n')
        snapshot=self.snapshot();self.write(target,'[Desktop Entry]\nExec=/usr/bin/sleep 90\n')
        self.assertTrue(snapshot.changed())

    def test_container_defaults_are_pinned_not_empty(self):
        (self.user/'com.system76.CosmicPanel/v1/entries').unlink()
        with self.assertRaisesRegex(ValueError,'unqualified_configured'):self.snapshot()
        ids=['CosmicPanelWorkspacesButton','CosmicPanelAppButton','CosmicAppletTime','CosmicAppletInputSources','CosmicAppletA11y','CosmicAppletStatusArea','CosmicAppletTiling','CosmicAppletAudio','CosmicAppletBluetooth','CosmicAppletNetwork','CosmicAppletBattery','CosmicAppletNotifications','CosmicAppletPower','CosmicPanelLauncherButton','CosmicAppList','CosmicAppletMinimize']
        for id in ids:
            id='com.system76.'+id;self.policy['applets'][id]=dict(process='audio',args=['90'])
            self.write(self.data/('applications/'+id+'.desktop'),'[Desktop Entry]\nExec=/usr/bin/sleep 90\n')
        self.assertEqual(self.snapshot().counts,{'audio':18})

    def test_empty_core_only_policy_cannot_conceal_configured_children(self):
        self.policy['applets']={};self.policy['processes']={}
        with self.assertRaisesRegex(ValueError,'unqualified_configured'):self.snapshot()
        self.setbar('Panel',[]);self.assertEqual(self.snapshot().counts,{})

    def test_ron_and_desktop_supported_escapes(self):
        self.assertEqual(ron('/* outer /*nested*/ */ Some((["Audio",],[])) //end'),('variant','Some',(['Audio'],[])))
        self.assertEqual(desktop('[Desktop Entry]\nName= hello\\sworld\nExec=sleep 90\n')['Name'],'hello world')
        with self.assertRaises(ValueError):desktop('[Desktop Entry]\nExec=sleep\\')

class WaylandInventory(unittest.TestCase):
    def test_real_socket_registry_names_and_protocol_rejection(self):
        for version in (4,3):
            with self.subTest(version=version),tempfile.TemporaryDirectory() as tmp:
                path=Path(tmp)/'wayland';server=socket.socket(socket.AF_UNIX);server.bind(str(path));server.listen(1)
                errors=[]
                def string(s):
                    b=s.encode()+b'\0';return struct.pack('I',len(b))+b+b'\0'*((-len(b))%4)
                def serve():
                    try:
                        with server.accept()[0] as c:
                            c.settimeout(2)
                            def recv(n):
                                data=b''
                                while len(data)<n:
                                    chunk=c.recv(n-len(data))
                                    if not chunk:return None
                                    data+=chunk
                                return data
                            def send(obj,op,b):c.sendall(struct.pack('II',obj,((len(b)+8)<<16)|op)+b)
                            for _ in range(8):
                                header=recv(8)
                                if header is None:return
                                obj,word=struct.unpack('II',header);b=recv((word>>16)-8)
                                if obj==1 and word&65535==1:
                                    send(2,0,struct.pack('I',7)+string('wl_output')+struct.pack('I',version))
                                elif obj==1:send(struct.unpack('I',b)[0],0,struct.pack('I',0))
                                elif obj==2:send(struct.unpack_from('I',b,len(b)-4)[0],4,string('DP-1'))
                    except (BrokenPipeError,ConnectionResetError):pass
                    except Exception as e:errors.append(repr(e))
                thread=threading.Thread(target=serve);thread.start()
                try:
                    if version==4:self.assertEqual(output_names({'WAYLAND_DISPLAY':str(path)}),['DP-1'])
                    else:
                        with self.assertRaisesRegex(ValueError,'output_scope'):output_names({'WAYLAND_DISPLAY':str(path)})
                finally:thread.join(3);server.close()
                self.assertFalse(thread.is_alive());self.assertEqual(errors,[])

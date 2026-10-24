"""Actual standalone manual install output; no helper/repository import path."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
import install

PROBE = r'''
import json,runpy,sys
module=runpy.run_path(sys.argv[1],run_name='isolated_launcher')
main=module['main'];calls=[]
main.__globals__['other_host']=lambda:None
main.__globals__['stock_host']=lambda *a,**k:calls.append('stock') or 0
print(json.dumps({'result':main(),'dispatch':calls}))
'''

class ManualFallback(unittest.TestCase):
    def test_actual_copied_legacy_and_unusable_v2_reach_stock_without_helpers(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);home=root/'home';home.mkdir();runtime=root/'runtime';runtime.mkdir()
            binary=root/'binary';binary.write_bytes(b'not executed')
            manifest=root/'manifest';record=dict(upstream='ca20ce3185b3b4404f429d59d429c40b58a2ddef',protocol=1,binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),runtime_probe=1)
            manifest.write_text(json.dumps(record));install.install(home,binary,manifest,enable=True)
            launcher=home/'.local/bin/cosmic-panel';bundle=home/'.local/share/cosmic-desktop-cards/companion'
            self.assertFalse((bundle/'bundle_contract.py').exists())
            env={**os.environ,'HOME':str(home),'XDG_DATA_HOME':str(home/'.local/share'),'XDG_STATE_HOME':str(home/'.local/state'),'XDG_RUNTIME_DIR':str(runtime)}
            for key in ('PYTHONPATH','WAYLAND_SOCKET','PANEL_NOTIFICATIONS_FD'):env.pop(key,None)
            for version in ('legacy','unusable-v2'):
                with self.subTest(version=version):
                    if version=='unusable-v2':
                        (bundle/'manifest.json').write_text(json.dumps({**record,'format':'halcyon-islands-bundle','version':2}))
                    result=subprocess.run([sys.executable,'-I','-c',PROBE,str(launcher)],cwd=root,env=env,text=True,capture_output=True,check=True)
                    self.assertEqual(json.loads(result.stdout),dict(result=0,dispatch=['stock']))
                    self.assertFalse((home/'.local/state/desktop-cards-panel/failed').exists())

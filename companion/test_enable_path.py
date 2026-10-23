"""Execute the real v2 CLI in an isolated home, with only host attestations replaced."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from bundle_contract import canonical
from test_generation_contract import bundle_fixture


class EnablePath(unittest.TestCase):
    def test_production_cli_missing_reversed_and_normal_path(self):
        cli=Path(__file__).resolve().parents[1]/'packaging/islands/halcyon-islands'
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary);library=root/'library';manifest,catalog=bundle_fixture(library)
            harness=root/'cli-harness.py'
            harness.write_text('''import os,sys,runpy,subprocess
from pathlib import Path
library=Path(sys.argv[2]);sys.path.insert(0,str(library))
import refresh,runtime_identity
# No production bypass: these substitutions exist only in this isolated test.
refresh.LIBRARY=library
real_verify=refresh.verify_bundle
refresh.verify_bundle=lambda path,*args,**kw: real_verify(path,*args,**{**kw,'owner':os.getuid()})
cohort=sys.argv[3]
runtime_identity.installed_identity=lambda _: {'verified':True,'cohort':cohort}
real_run=subprocess.run
subprocess.run=lambda args,**kw: subprocess.CompletedProcess(args,0,'install ok installed\\n0.2.1','') if args[0]=='/usr/bin/dpkg-query' else real_run(args,**kw)
entry=runpy.run_path(sys.argv[1]);entry['main'].__globals__['LIBRARY']=library
sys.argv=[sys.argv[1],'enable']
sys.exit(entry['main']())
''')
            for case in ('missing','reversed','normal'):
                with self.subTest(case=case):
                    home=root/case;home.mkdir();local=str(home/'.local/bin')
                    path={'missing':'/usr/bin','reversed':'/usr/bin:'+local,'normal':local+':/usr/bin'}[case]
                    env={**os.environ,'HOME':str(home),'XDG_DATA_HOME':str(home/'.local/share'),'XDG_CONFIG_HOME':str(home/'.config'),'XDG_STATE_HOME':str(home/'.local/state'),'XDG_CURRENT_DESKTOP':'COSMIC','XDG_SESSION_ID':'isolated-test','PATH':path}
                    run=subprocess.run([sys.executable,str(harness),str(cli),str(library),catalog['cohorts'][0]['id']],env=env,text=True,capture_output=True,timeout=20)
                    if case=='normal':
                        self.assertEqual(run.returncode,0,run.stderr)
                        self.assertEqual(json.loads(run.stdout)['state'],'staged')
                        self.assertEqual((home/'.local/bin/cosmic-panel').read_bytes(),(library/'selector.py').read_bytes())
                    else:
                        self.assertNotEqual(run.returncode,0)
                        self.assertIn('PATH must put ~/.local/bin before /usr/bin',run.stderr)
                        self.assertEqual(list(home.iterdir()),[], 'precondition creates no lock/journal or shell configuration')
                    self.assertFalse((home/'.profile').exists())
                    self.assertFalse((home/'.bashrc').exists())

if __name__=='__main__':unittest.main()

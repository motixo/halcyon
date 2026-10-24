#!/usr/bin/python3
"""Isolated synthetic fork lifetime check; no desktop or host service changes."""
import json,os,pathlib,signal,subprocess,sys,tempfile,time
script='''import os,time,sys
pid=os.fork()
if pid:
 print(pid,flush=True);os._exit(0)
else:
 time.sleep(15)
'''
p=subprocess.Popen([sys.executable,'-c',script],stdout=subprocess.PIPE,start_new_session=True,text=True)
child=int(p.stdout.readline());p.wait(timeout=3)
try:
 info=pathlib.Path(f'/proc/{child}/stat').read_text();assert p.returncode==0 and info.rsplit(')',1)[1].split()[0]!='Z'
 print(json.dumps({'synthetic_parent_exit':p.returncode,'child_survives_parent_exit':True,'supervisor_must_follow_forked_child':True,'actual_systemd_and_native_canary_still_required':True}))
finally:
 os.killpg(p.pid,signal.SIGTERM)

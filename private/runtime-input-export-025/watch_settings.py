import pathlib,sys,json,subprocess,time,os
from guest_session import session_env
D=pathlib.Path(__file__).resolve().parent;O=D/'results';label=sys.argv[2];binary=sys.argv[3]
if sys.argv[1]=='start':
 p=subprocess.Popen(['/usr/bin/python3','-B',__file__,'worker',label,binary],stdin=subprocess.DEVNULL,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,start_new_session=True);print(json.dumps({'watcher_pid':p.pid}));raise SystemExit(0)
args=['/usr/sbin/runuser','-u','probe','--','/usr/bin/env','-i',*[k+'='+v for k,v in session_env().items()],binary,'--settings','general']
start=time.time();p=subprocess.Popen(args,stdin=subprocess.DEVNULL,stdout=(O/(label+'.stdout')).open('wb'),stderr=(O/(label+'.stderr')).open('wb'));(O/(label+'.launch.json')).write_text(json.dumps({'pid':p.pid,'binary':binary,'start':start}))
try:rc=p.wait(timeout=300);result={'returncode':rc,'seconds':time.time()-start,'normal_close_requested_by_ui':True}
except subprocess.TimeoutExpired:result={'error':'ordinary close did not finish within 300 seconds','still_running':True};# preserve, do not force crash or hunt core
(O/(label+'.exit.json')).write_text(json.dumps(result,indent=2)+'\n')

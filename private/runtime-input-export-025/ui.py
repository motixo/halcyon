"""One offline VM, real stock input, bounded timing probes; never qualification."""
import base64,hashlib,json,os,pathlib,shutil,socket,subprocess,tarfile,time,traceback
from common import HERE,sha,pins
from vm import W,E,R,Channel,guest,put,boot,stop
from PIL import Image
T=W/'trial';Q=None

def record(event,**kw):
 with (E/'ui-timeline.jsonl').open('a') as f:f.write(json.dumps(dict(event=event,unix_time=time.time(),monotonic=time.monotonic(),**kw))+'\n')
def qmp(cmd,args=None):return Q.call(cmd,args)
def shot(name):
 path=E/(name+'.ppm');qmp('screendump',{'filename':str(path)});Image.open(path).save(E/(name+'.png'));path.unlink();record('screenshot',name=name)
def key(keys,delay=.12):
 record('key',keys=keys);qmp('human-monitor-command',{'command-line':'sendkey '+keys});time.sleep(delay)
def text(s):
 for ch in s:key({'-':'minus',' ':'spc'}.get(ch,ch))
def click(x,y):
 record('pointer',x=x,y=y);qmp('input-send-event',{'events':[{'type':'abs','data':{'axis':'x','value':round(x*32767/1280)}},{'type':'abs','data':{'axis':'y','value':round(y*32767/800)}},{'type':'btn','data':{'down':True,'button':'left'}}]});time.sleep(.08);qmp('input-send-event',{'events':[{'type':'btn','data':{'down':False,'button':'left'}}]})
def exec_ok(path,args,timeout=60):
 rc,out,err=guest(T,path,args,timeout)
 if rc:raise RuntimeError('guest command failed: '+path+' '+err.decode(errors='replace')[-1000:])
 return out
def interaction(label):
 shot(label+'-before');click(640,16);time.sleep(.25);shot(label+'-clock-250ms');time.sleep(.75);shot(label+'-clock-1s');click(960,650);time.sleep(.3)
 key('meta_l');time.sleep(.3);text('terminal');time.sleep(.25);shot(label+'-typed-250ms');time.sleep(.75);shot(label+'-typed-1s');key('esc');time.sleep(.3)
def getfile(src,dst):
 c=Channel(T/'agent.sock');h=c.call('guest-file-open',{'path':src,'mode':'r'});n=0
 try:
  with dst.open('xb') as f:
   while True:
    r=c.call('guest-file-read',{'handle':h,'count':32768});b=base64.b64decode(r.get('buf-b64',''));n+=len(b)
    if n>150*1024**2:raise RuntimeError('guest evidence size bound')
    f.write(b)
    if r.get('eof'):break
 finally:c.call('guest-file-close',{'handle':h});c.close()
 return n

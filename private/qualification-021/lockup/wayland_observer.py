#!/usr/bin/python3
"""Read-only real compositor toplevel observer. No activation/input requests."""
import json,os,pathlib,select,socket,struct,time,xml.etree.ElementTree as ET

def u(n):return struct.pack('<I',n)
def string(value):
 b=value.encode()+b'\0';return u(len(b))+b+b'\0'*((-len(b))%4)
def scalar(data,offset=0):return struct.unpack_from('<I',data,offset)[0]
def schema(directory):
 out={}
 for path in pathlib.Path(directory).glob('*.xml'):
  for interface in ET.parse(path).getroot().findall('interface'):
   out[interface.attrib['name']]=[[a.attrib for a in e.findall('arg')] for e in interface.findall('event')]
 return out
class Observer:
 def __init__(self,directory):
  self.schemas=schema(directory);self.ids={1:'wl_display'};self.next_id=2;self.buffer=b'';self.globals=[];self.foreign={};self.extended={};self.links={};self.outputs={};self.done=set();self.info=None
  self.sock=socket.socket(socket.AF_UNIX);self.sock.settimeout(5)
  display=os.environ.get('WAYLAND_DISPLAY','wayland-0');path=display if display.startswith('/') else str(pathlib.Path(os.environ['XDG_RUNTIME_DIR'])/display);self.sock.connect(path)
 def new(self,interface):
  n=self.next_id;self.next_id+=1;self.ids[n]=interface;return n
 def send(self,obj,opcode,payload=b''):
  self.sock.sendall(u(obj)+u(((len(payload)+8)<<16)|opcode)+payload)
 def bind(self,global_,version):
  n=self.new(global_['interface']);self.send(self.registry,0,u(global_['name'])+string(global_['interface'])+u(min(version,global_['version']))+u(n));return n
 def roundtrip(self):
  n=self.new('wl_callback');self.send(1,0,u(n));end=time.monotonic()+5
  while n not in self.done and time.monotonic()<end:self.receive()
  if n not in self.done:raise TimeoutError('real compositor roundtrip')
 def receive(self):
  data=self.sock.recv(65536)
  if not data:raise ConnectionError('Compositor closed observer connection')
  self.buffer+=data
  while len(self.buffer)>=8:
   obj,word=struct.unpack_from('<II',self.buffer);size=word>>16;opcode=word&65535
   if size<8:raise RuntimeError('Invalid Wayland frame')
   if len(self.buffer)<size:return
   data=self.buffer[8:size];self.buffer=self.buffer[size:];interface=self.ids.get(obj)
   if interface=='wl_display':
    if opcode==0:raise RuntimeError('Wayland protocol error: '+repr(data))
   elif interface=='wl_callback':self.done.add(obj)
   elif interface=='wl_registry' and opcode==0:
    length=scalar(data,4);self.globals.append({'name':scalar(data),'interface':data[8:8+length-1].decode(),'version':scalar(data,8+((length+3)//4)*4)})
   elif interface=='wl_output':pass
   elif interface in self.schemas:
    args=self.schemas[interface][opcode];values=[];pos=0
    for a in args:
     t=a['type']
     if t in ('string','array'):
      length=scalar(data,pos);pos+=4;raw=data[pos:pos+length];pos+=(length+3)//4*4;values.append(raw[:-1].decode() if t=='string' and length else (list(struct.unpack('<'+'I'*(length//4),raw)) if t=='array' else ''))
     else:
      val=struct.unpack_from('<i' if t=='int' else '<I',data,pos)[0];pos+=4;values.append(val)
      if t=='new_id':self.ids[val]=a['interface']
    if interface=='ext_foreign_toplevel_list_v1' and opcode==0:
     handle=values[0];self.foreign[handle]={'id':handle,'closed':False};extension=self.new('zcosmic_toplevel_handle_v1');self.links[extension]=handle;self.extended[extension]={};self.send(self.info,1,u(extension)+u(handle))
    elif interface=='ext_foreign_toplevel_handle_v1':
     if opcode==0:self.foreign[obj]['closed']=True
     elif opcode==2:self.foreign[obj]['title']=values[0]
     elif opcode==3:self.foreign[obj]['app_id']=values[0]
    elif interface=='zcosmic_toplevel_handle_v1':
     if opcode==8:self.extended[obj]['states']=values[0]
     elif opcode==9:self.extended[obj]['geometry']=dict(zip(['output','x','y','width','height'],values))
 def wait_initial(self,timeout=3):
  # Initial extension properties are emitted from compositor refresh(), not
  # guaranteed before wl_display.sync. Keep this connection alive until they
  # arrive; recreating connections around immediate roundtrips loses them.
  started=time.monotonic();end=started+timeout
  while time.monotonic()<end:
   live=[ext for ext,foreign in self.links.items() if not self.foreign[foreign].get('closed')]
   complete=all('states' in self.extended[ext] and 'geometry' in self.extended[ext] for ext in live)
   if complete and time.monotonic()-started>=.10:return True
   ready,_,_=select.select([self.sock],[],[],min(.05,max(0,end-time.monotonic())))
   if ready:self.receive()
  return False
 def snapshot(self):
  self.registry=self.new('wl_registry');self.send(1,1,u(self.registry));self.roundtrip()
  for g in self.globals:
   if g['interface']=='wl_output':self.outputs[self.bind(g,4)]=g['name']
  self.info=self.bind(next(g for g in self.globals if g['interface']=='zcosmic_toplevel_info_v1'),3)
  self.bind(next(g for g in self.globals if g['interface']=='ext_foreign_toplevel_list_v1'),1)
  for _ in range(3):self.roundtrip()
  initial_complete=self.wait_initial()
  for ext,foreign in self.links.items():self.foreign[foreign].update(self.extended[ext])
  return {'time':time.time(),'globals':self.globals,'windows':list(self.foreign.values()),'read_only':True,'initial_extension_complete':initial_complete}
if __name__=='__main__':
 observer=Observer(pathlib.Path(__file__).parent/'protocols')
 try:print(json.dumps(observer.snapshot()))
 finally:observer.sock.close()

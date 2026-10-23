#!/usr/bin/python3
"""Local wire/framing/media preflight. Never connects to host desktop."""
import ast,importlib.util,json,pathlib,socket,struct,unittest,threading,time,sys
sys.path.insert(0,str(pathlib.Path(__file__).resolve().parent.parent/'theme'))
ROOT=pathlib.Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('observer',ROOT/'wayland_observer.py');m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
class Wire(unittest.TestCase):
 def setUp(self):
  self.a,self.b=socket.socketpair();self.o=m.Observer.__new__(m.Observer);self.o.sock=self.a;self.o.schemas=m.schema(ROOT/'protocols');self.o.ids={4:'ext_foreign_toplevel_handle_v1',5:'zcosmic_toplevel_handle_v1'};self.o.buffer=b'';self.o.foreign={4:{'id':4,'closed':False}};self.o.extended={5:{}}
 def tearDown(self):self.a.close();self.b.close()
 def frame(self,obj,opcode,data):return m.u(obj)+m.u(((len(data)+8)<<16)|opcode)+data
 def test_fragmented_title_state_geometry(self):
  frames=self.frame(4,2,m.string('lockup-input.txt'))+self.frame(5,8,m.u(4)+m.u(2))+self.frame(5,9,struct.pack('<Iiiii',9,-10,20,800,500))
  self.b.sendall(frames[:13]);self.o.receive();self.assertEqual(self.o.foreign[4].get('title'),None)
  self.b.sendall(frames[13:]);self.o.receive();self.assertEqual(self.o.foreign[4]['title'],'lockup-input.txt');self.assertEqual(self.o.extended[5]['states'],[2]);self.assertEqual(self.o.extended[5]['geometry']['x'],-10)
 def test_delayed_extension_initial_events(self):
  self.o.links={5:4}
  def compositor_refresh():
   time.sleep(.05)
   self.b.sendall(self.frame(5,8,m.u(4)+m.u(2))+self.frame(5,9,struct.pack('<Iiiii',9,212,116,856,464)))
  t=threading.Thread(target=compositor_refresh);t.start()
  started=time.monotonic();self.assertTrue(self.o.wait_initial(.5));t.join()
  self.assertGreaterEqual(time.monotonic()-started,.09);self.assertEqual(self.o.extended[5]['states'],[2]);self.assertEqual(self.o.extended[5]['geometry']['width'],856)
 def test_incomplete_extension_is_not_complete(self):
  self.o.links={5:4};self.assertFalse(self.o.wait_initial(.03))
 def test_retained_editor_pixel_focus(self):
  spec=importlib.util.spec_from_file_location('editor_pixels',ROOT/'editor_pixels.py');pixel=importlib.util.module_from_spec(spec);spec.loader.exec_module(pixel)
  retained=ROOT/'evidence-40015446836/lockup-session-stock-40015446836/001-failure.png'
  if not retained.exists():self.skipTest('Retained local screen not embedded in CI checkout')
  box=pixel.locate(retained,[],True)
  self.assertTrue(210<=box['x']<=215 and 112<=box['y']<=120);self.assertTrue(850<=box['width']<=865)
  missing=ROOT/'evidence-40013237899/lockup-session-serial-40013237899/stock/001-failure.png'
  with self.assertRaises(ValueError):pixel.locate(missing,[],True)
 def test_read_only_requests(self):
  source=(ROOT/'wayland_observer.py').read_text();self.assertNotIn('activate',source.replace('activation',''));self.o.send(3,1,m.u(4));self.assertEqual(self.b.recv(99),self.frame(3,1,m.u(4)))
 def test_preflight_scope(self):
  s=(ROOT/'start.sh').read_text();self.assertIn('-nic none',s);self.assertIn('readonly=on,file=$WORK/candidate.iso',s);self.assertIn('$COHORT != stock',s);self.assertNotIn('/home/workstation',s)
  for path in ROOT.glob('*.py'):ast.parse(path.read_text())
  s=(ROOT/'session.py').read_text();self.assertIn("h.install('public')",s);self.assertNotIn("h.install('candidate')",s);self.assertNotIn('color_scheme',s)
if __name__=='__main__':unittest.main()

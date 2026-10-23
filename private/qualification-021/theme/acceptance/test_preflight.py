#!/usr/bin/python3
"""No VM/UI execution. Test the actual launch builder and media identity guard."""
import importlib,json,pathlib,sys,unittest
ROOT=pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT))
import acceptance_wrapper as w
s=w
class PreflightTests(unittest.TestCase):
 def test_every_real_phase_builds_expected_readonly_media(self):
  for host in ['stock','islands']:
   for phase in w.plan(host):
    args=w.qemu_args(pathlib.Path('/ci/root'),pathlib.Path('/ci/phase'),phase)
    inputs=[v for v in args if 'id=media,' in v]
    self.assertEqual(bool(inputs),phase.lineage=='candidate',phase.name)
    if inputs:self.assertIn('readonly=on,file=/ci/root/candidate.iso',inputs[0])
    self.assertEqual(args[args.index('-nic')+1],'none')
 def test_baseline_first_and_independent_islands_manual(self):
  self.assertEqual(w.plan('stock')[0].name,'untouched-clock')
  self.assertEqual([p.task for p in w.plan('islands')],['manual','natural'])
  self.assertTrue(all(p.media for p in w.plan('islands')))
 def test_empty_implicit_dvd_is_allowed_for_untouched(self):
  self.assertTrue(s.media_assertion([{'device':'ide2-cd0','removable':True,'tray_open':False},{'device':'osdisk','removable':False,'inserted':{'file':'trial.qcow2','ro':False}}],None)['asserted'])
 def test_inserted_media_is_rejected_for_untouched(self):
  with self.assertRaisesRegex(RuntimeError,'inserted'):s.media_assertion([{'device':'ide2-cd0','removable':True,'inserted':{'file':'foreign.iso','ro':True}}],None)
 def test_candidate_identity_and_readonly_required(self):
  for filename,ro in [('other.iso',True),('/ci/candidate.iso',False)]:
   with self.assertRaisesRegex(RuntimeError,'identity'):s.media_assertion([{'device':'media','removable':True,'inserted':{'file':filename,'ro':ro}}],'/ci/candidate.iso')
  self.assertTrue(s.media_assertion([{'device':'media','removable':True,'inserted':{'file':'/ci/candidate.iso','ro':True}}],'/ci/candidate.iso')['asserted'])
 def test_solar_coordinates_and_matched_native_lock(self):
  solar=json.loads((ROOT/'acceptance/solar-boundary.json').read_text());proof=json.loads((ROOT/'acceptance/solar-provenance.json').read_text())
  self.assertEqual((solar['latitude'],solar['longitude']),(51.50853,-0.12574))
  self.assertEqual(solar['sunrise_epoch'],1792737461)
  self.assertEqual(proof['matched_native_dependencies']['sunrise']['version'],'2.1.0')
if __name__=='__main__':unittest.main()

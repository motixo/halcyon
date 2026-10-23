#!/usr/bin/python3
"""Local regression against actual retained screenshots; never runs an app."""
import json,pathlib,unittest
from calibrated_controls import point,Pixels,LOOK_BOUNDS
ROOT=pathlib.Path(__file__).parent
CAL=json.loads((ROOT/'followup/control-calibration.json').read_text())
class CalibrationTests(unittest.TestCase):
 def test_ocr_missed_calm_native_controls_are_identified(self):
  path=pathlib.Path(CAL['sources']['native_calm']['path'])
  self.assertEqual(point(path,'Dark',CAL)['point'],[673,347])
  self.assertEqual(point(path,'Light',CAL)['point'],[895,347])
  self.assertFalse(point(path,'Auto',CAL)['enabled'])
 def test_native_stock_and_calm_share_verified_control_geometry(self):
  for k in ['native_stock','native_calm']:
   for mode in ['Dark','Light','Auto']:point(pathlib.Path(CAL['sources'][k]['path']),mode,CAL)
 def test_all_five_look_points_are_inside_actual_preserved_tiles(self):
  path=pathlib.Path(CAL['sources']['wizard']['path'])
  for look,bounds in LOOK_BOUNDS.items():
   p=point(path,look,CAL)['point'];self.assertTrue(bounds[0]<p[0]<bounds[2] and bounds[1]<p[1]<bounds[3])
 def test_native_light_image_preserves_control_calibration(self):
  p=ROOT/'evidence-39974915851/theme-session-candidate-islands-39974915851/009-native-selected-Light.png'
  self.assertEqual(point(p,'Dark',CAL)['point'],[673,347])
  self.assertEqual(point(p,'Light',CAL)['point'],[895,347])
 def test_wizard_calibration_survives_actual_selection_and_host_variants(self):
  for source in CAL['wizard_validation_sources']:
   self.assertGreaterEqual(point(pathlib.Path(source['path']),'calm',CAL)['matched_samples'],43)
 def test_welcome_button_identified_without_sparse_ocr(self):
  self.assertEqual(point(pathlib.Path(CAL['sources']['welcome']['path']),'Start',CAL)['point'],[345,451])
 def test_wrong_screen_is_rejected_before_click(self):
  with self.assertRaisesRegex(ValueError,'pattern absent'):point(pathlib.Path(CAL['sources']['wizard']['path']),'Dark',CAL)
  with self.assertRaisesRegex(ValueError,'differs'):point(pathlib.Path(CAL['sources']['native_calm']['path']),'calm',CAL)
if __name__=='__main__':unittest.main()

import unittest
from configuration import FLAGS,fields,change_flags,verify_media

SAMPLE='''(\n    palette: Dark((some_color: "#ABCDEF",)),\n    frosted: Medium,\n    frosted_windows: false,\n    frosted_system_interface: false,\n    frosted_panel: false,\n    frosted_applets: false,\n    alpha_map: (medium: 0.76154,),\n)\n'''
class Limits(unittest.TestCase):
    def test_roundtrip_keeps_unrelated_export_bytes(self):
        enabled,proof=change_flags(SAMPLE,True)
        restored,_=change_flags(enabled,False)
        self.assertEqual(restored,SAMPLE)
        self.assertTrue(all(fields(enabled)[k]=='true' for k in FLAGS))
        self.assertIn('frosted_panel: false',enabled)
        self.assertIn('alpha_map: (medium: 0.76154,)',enabled)
    def test_zero_strength_is_not_equivalent_off(self):
        with self.assertRaises(ValueError):fields(SAMPLE.replace('Medium','ExtremelyLow'))
    def test_ambiguous_native_export_refused(self):
        with self.assertRaises(ValueError):change_flags(SAMPLE.replace('    frosted_windows:', '    frosted_windows: false,\n    frosted_windows:'),True)
    def test_missing_flag_refused(self):
        with self.assertRaises(ValueError):change_flags(SAMPLE.replace('    frosted_applets: false,\n',''),True)
    def test_empty_dvd_node_is_valid_stock(self):
        verify_media([{'removable':True,'device':'ide2-cd0'}],False)
    def test_public_media_identity_and_readonly_required(self):
        verify_media([{'removable':True,'inserted':{'ro':True,'file':'/fixture/candidate.iso'}}],True)
        with self.assertRaises(ValueError):verify_media([{'removable':True,'inserted':{'ro':False,'file':'/fixture/candidate.iso'}}],True)
        with self.assertRaises(ValueError):verify_media([{'removable':True,'inserted':{'ro':True,'file':'/fixture/candidate.iso'}}],False)
    def test_light_mode_cannot_silently_replace_dark(self):
        with self.assertRaises(ValueError):fields(SAMPLE.replace('Dark(', 'Light('))
if __name__=='__main__':unittest.main()

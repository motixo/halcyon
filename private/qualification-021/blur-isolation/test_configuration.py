import unittest
from configuration import FLAGS,fields,change_flags,verify_media,validate_reexport,validate_stored_builder,export_values,ron_tokens

SAMPLE='''(\n    palette: Dark((some_color: "#ABCDEF",)),\n    frosted: Medium,\n    frosted_windows: false,\n    frosted_system_interface: false,\n    frosted_panel: false,\n    frosted_applets: false,\n    alpha_map: (medium: 0.76154,),\n)\n'''
class Limits(unittest.TestCase):
    def test_all_three_preparations_with_real_native_output(self):
        from pathlib import Path
        import json
        root=Path(__file__).parent/'fixtures'
        before=(root/'native-builder-before.ron').read_text();on,_=change_flags(before,True)
        stored=json.loads((root/'native-stored-builder.json').read_text())
        self.assertEqual(validate_stored_builder(on,stored)['count'],22)
        actual_on=(root/'native-builder-after.ron').read_text();off,_=change_flags(actual_on,False)
        a,b=export_values(actual_on),export_values(off)
        self.assertTrue(all(b[k]=='false' for k in FLAGS))
        self.assertTrue(all(ron_tokens(a[k])==ron_tokens(b[k]) for k in a if k not in FLAGS))
        public=json.loads((root/'public-r3-global-effects.json').read_text())['fields']
        self.assertEqual(len(public),12)
        self.assertTrue(all(v.strip()=='true' for v in public.values()))
    def test_real_native_storage_mutation_is_rejected(self):
        from pathlib import Path
        import json
        root=Path(__file__).parent/'fixtures';on,_=change_flags((root/'native-builder-before.ron').read_text(),True)
        stored=json.loads((root/'native-stored-builder.json').read_text());stored['frosted_windows']='false'
        with self.assertRaises(ValueError):validate_stored_builder(on,stored)

    def native_fixture(self):
        from pathlib import Path
        root=Path(__file__).parent/'fixtures'
        return [ (root/name).read_text() for name in ['native-builder-before.ron','native-builder-after.ron','stored-builder-text-tint.ron','generated-background.ron'] ]
    def test_real_native_projection_uses_actual_generated_foreground(self):
        before,after,stored,bg=self.native_fixture();desired,_=change_flags(before,True)
        self.assertEqual(validate_reexport(desired,after,stored,bg)['kind'],'native-derived-foreground')
    def test_other_native_palette_changes_are_rejected(self):
        before,after,stored,bg=self.native_fixture();desired,_=change_flags(before,True)
        with self.assertRaises(ValueError):validate_reexport(desired,after.replace('#63D0DFFF','#123456FF'),stored,bg)
    def test_raw_builder_input_cannot_be_overridden(self):
        before,after,stored,bg=self.native_fixture();desired,_=change_flags(before,True)
        with self.assertRaises(ValueError):validate_reexport(desired,after,'Some("#010101FF")',bg)
    def test_foreground_fixture_is_not_a_hardcoded_expected_palette(self):
        before,after,stored,bg=self.native_fixture();desired,_=change_flags(before,True)
        with self.assertRaises(ValueError):validate_reexport(desired,after,stored,bg.replace('#F5F5F5FF','#123456FF'))

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

import hashlib,pathlib,tempfile,unittest,zipfile
from common import extract_verified,purge_changes
HERE=pathlib.Path(__file__).parent
class Boundaries(unittest.TestCase):
 def fixture(self,names):
  tmp=tempfile.TemporaryDirectory(dir=HERE);self.addCleanup(tmp.cleanup);root=pathlib.Path(tmp.name);z=root/'in.zip'
  with zipfile.ZipFile(z,'w') as f:
   for name in names:f.writestr(name,b'known input')
  return z,root/'extract',{'disk':hashlib.sha256(b'known input').hexdigest()}
 def test_wrong_zip_hash_writes_nothing(self):
  z,d,p=self.fixture(['disk'])
  with self.assertRaisesRegex(ValueError,'ZIP identity'):extract_verified(z,d,z.stat().st_size,'0'*64,p)
  self.assertFalse(d.exists())
 def test_traversal_rejected_before_extraction(self):
  z,d,p=self.fixture(['../disk'])
  with self.assertRaisesRegex(ValueError,'unsafe'):extract_verified(z,d,z.stat().st_size,hashlib.sha256(z.read_bytes()).hexdigest(),p)
  self.assertFalse(d.exists())
 def test_missing_or_extra_original_object_rejected(self):
  z,d,p=self.fixture(['disk','extra'])
  with self.assertRaisesRegex(ValueError,'member set'):extract_verified(z,d,z.stat().st_size,hashlib.sha256(z.read_bytes()).hexdigest(),p)
  self.assertFalse(d.exists())
 def test_verified_exact_object_extracts_readonly(self):
  z,d,p=self.fixture(['disk']);r=extract_verified(z,d,z.stat().st_size,hashlib.sha256(z.read_bytes()).hexdigest(),p)
  self.assertEqual(r['disk']['sha256'],p['disk']);self.assertEqual((d/'disk').stat().st_mode&0o777,0o444)
 def test_purge_addition_rejected(self):
  with self.assertRaisesRegex(ValueError,'extra'):purge_changes('Purg old [1]\nInst new [2]\n',{'old'})
 def test_extra_removal_rejected(self):
  with self.assertRaisesRegex(ValueError,'extra'):purge_changes('Purg old [1]\nRemv another [2]\n',{'old'})
 def test_exact_removal_accepted(self):self.assertEqual(len(purge_changes('Purg old [1]\n',{'old'})),1)
if __name__=='__main__':unittest.main()

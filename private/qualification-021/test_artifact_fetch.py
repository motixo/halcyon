import hashlib,importlib.util,pathlib,tempfile,unittest,zipfile
HERE=pathlib.Path(__file__).parent
spec=importlib.util.spec_from_file_location('artifact_fetch',HERE/'artifact-fetch.py')
module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)

class ArtifactBoundary(unittest.TestCase):
    def fixture(self,name):
        t=tempfile.TemporaryDirectory(dir=HERE);self.addCleanup(t.cleanup)
        root=pathlib.Path(t.name);z=root/'input.zip'
        with zipfile.ZipFile(z,'w') as archive:archive.writestr(name,b'retained payload')
        digest='sha256:'+hashlib.sha256(z.read_bytes()).hexdigest()
        return z,root/'out',digest

    def test_wrong_digest_extracts_nothing(self):
        z,dest,digest=self.fixture('disk')
        with self.assertRaisesRegex(RuntimeError,'digest mismatch'):
            module.unpack_verified(z,dest,'sha256:'+'0'*64)
        self.assertFalse(dest.exists())

    def test_authenticated_zip_cannot_escape_destination(self):
        z,dest,digest=self.fixture('../escaped')
        with self.assertRaisesRegex(RuntimeError,'Unsafe'):
            module.unpack_verified(z,dest,digest)
        self.assertFalse((z.parent/'escaped').exists())

    def test_verified_regular_file_is_extracted(self):
        z,dest,digest=self.fixture('retain/disk')
        module.unpack_verified(z,dest,digest)
        self.assertEqual((dest/'retain/disk').read_bytes(),b'retained payload')

if __name__=='__main__':unittest.main()

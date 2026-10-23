import datetime as dt
import hashlib
import json
from pathlib import Path
import sys
import unittest
sys.path.insert(0,str(Path(__file__).parent))
import discover

HERE=Path(__file__).parent
class SignedDiscovery(unittest.TestCase):
    def setUp(self):
        self.signed=(HERE/'fixtures/pop-InRelease').read_bytes()
        self.key=HERE/'trust/pop-archive.gpg'
        self.fingerprint='63C46DF0140D738961429F4E204DD8AEC33A7AFF'
        self.now=dt.datetime(2026,10,23,tzinfo=dt.timezone.utc)
    def verify(self,data=None,now=None,fingerprint=None):
        return discover.verify_release(data or self.signed,self.key,fingerprint or self.fingerprint,now or self.now)
    def test_actual_official_signature_and_bounded_expiry(self):
        release,hashes,expires=self.verify()
        self.assertEqual(release['Codename'],'noble');self.assertIn('main/binary-amd64/Packages.gz',hashes)
        self.assertTrue(expires.startswith('2026-10-28'))
    def test_tampered_wrong_key_expired_future_fail_closed(self):
        with self.assertRaises(ValueError):self.verify(self.signed.replace(b'Codename: noble',b'Codename: other'))
        with self.assertRaises(ValueError):self.verify(fingerprint='0'*40)
        with self.assertRaisesRegex(ValueError,'expired'):self.verify(now=self.now+dt.timedelta(days=20))
        with self.assertRaisesRegex(ValueError,'date_invalid'):self.verify(now=self.now-dt.timedelta(days=20))
    def test_index_package_same_source_altered_bytes_rejected(self):
        content=b'Package: cosmic-panel\nVersion: same-source\n'
        digest=hashlib.sha256(content).hexdigest()
        self.assertEqual(discover.checked(content,digest,len(content)),content)
        for value in (content+b'x',content.replace(b'panel',b'xxxxx')):
            with self.assertRaises(ValueError):discover.checked(value,digest,len(content))
    def test_unknown_control_syntax_and_archive_escape_rejected(self):
        with self.assertRaises(ValueError):discover.paragraphs(b'Package: x\nPackage: y\n')
        for path in ('../evil','https://elsewhere/a','/root','x?secret','x#fragment'):
            with self.assertRaises(ValueError):discover.child_url('https://apt.pop-os.org/release',path)
    def test_partial_dependency_and_virtual_version_constraints(self):
        installed={'panel':dict(version='1',depends='lib (>= 2) | other, virtual (= 4)'),
                   'lib':dict(version='1'), 'provider':dict(version='1',provides='virtual (= 4)')}
        self.assertTrue(discover.dependency_gaps(installed))
        installed['lib']['version']='2'
        self.assertEqual(discover.dependency_gaps(installed),[])
        installed['provider']['provides']='virtual'
        self.assertTrue(discover.dependency_gaps(installed))

    def test_event_identity_not_timestamp_deduplicates(self):
        from bundle_contract import identity
        event=dict(candidate='a'*64,reasons=['missing_complete_image'])
        self.assertEqual(identity(event),identity(dict(reasons=event['reasons'],candidate='a'*64)))
        self.assertNotEqual(identity(event),identity({**event,'candidate':'b'*64}))

if __name__=='__main__':unittest.main()

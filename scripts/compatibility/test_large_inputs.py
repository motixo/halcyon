"""Streaming/resource regressions, including actual signed records; no network.

Small scaled bounds exercise large-object behavior without gigabyte allocation.
Synthetic .debs are structural fixtures, never native qualification evidence.
"""
from contextlib import contextmanager
from dataclasses import replace
import datetime as dt
import hashlib
import io
import json
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import unittest
from unittest.mock import patch

import archive_io as aio
import discover
import catalog_capacity as capacity
from transport_test_support import response_worker

FIXTURE = Path(__file__).parent / 'fixtures'


def sha(value):
    return hashlib.sha256(value).hexdigest()


def deb(members, trailer=b''):
    output = io.BytesIO()
    with tarfile.open(fileobj=output, mode='w', format=tarfile.GNU_FORMAT) as archive:
        for name, content in members:
            member = tarfile.TarInfo(name); member.size = len(content)
            archive.addfile(member, io.BytesIO(content))
    return deb_tar(output.getvalue() + trailer)


def deb_tar(data):
    output = bytearray(b'!<arch>\n')
    for name, body in [('debian-binary', b'2.0\n'), ('control.tar', b'\0'*10240), ('data.tar', data)]:
        header = f'{name + "/":<16}{0:<12}{0:<6}{0:<6}{"100644":<8}{len(body):<10}`\n'.encode()
        assert len(header) == 60
        output.extend(header + body + (b'\n' if len(body) % 2 else b''))
    return bytes(output)


class Streaming:
    def __init__(self, data):
        self.data, self.reads, self.opens = data, [], 0

    def __call__(self, url):
        raise AssertionError('payload must use streaming open')

    @contextmanager
    def open(self, url):
        self.opens += 1
        source = io.BytesIO(self.data)
        owner = self
        class Reader:
            def read(self, size):
                owner.reads.append(size)
                assert 0 < size <= aio.CHUNK
                return source.read(size)
        yield Reader()


class LargeInputs(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(); self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.limits = aio.Limits()

    @classmethod
    def signed_rows(cls):
        path = FIXTURE/'multipocket'
        config = json.loads((path/'config.json').read_text())
        source = config['archives'][0]
        get = discover.retained_fetch(path/'inputs.json')
        base = source['url'] + '/dists/noble'
        _, hashes, _ = discover.verify_release(get(base+'/InRelease'), path/source['keyring'], source['fingerprint'], dt.datetime(2026,10,23,18,tzinfo=dt.timezone.utc))
        binary = discover.download_index(base,'main/binary-amd64/Packages.gz',hashes,get)
        sources = discover.download_index(base,'main/source/Sources.gz',hashes,get)
        return binary, sources

    def test_real_signed_large_records_use_signed_size_before_io(self):
        binary, sources = self.signed_rows()
        firmware = next(r for r in binary if r['Package']=='linux-firmware')
        source = next(r for r in sources if r['Package']=='linux-firmware')
        checksum,size,_ = next(line.split() for line in source['Checksums-Sha256'].splitlines() if line.endswith('.tar.xz'))
        self.assertEqual(int(firmware['Size']),1875589312)
        self.assertEqual(int(size),871086992)
        for digest,size,kind in ((firmware['SHA256'],int(firmware['Size']),'binary'), (checksum,int(size),'source-archive')):
            aio.signed_object(digest,size,kind,self.limits)
            stream=Streaming(b'')
            with self.assertRaisesRegex(ValueError,'archive_hash_mismatch'):
                aio.verify_stream(stream,'https://fixture/input',digest,size,kind,self.limits)
            self.assertEqual(stream.opens,1)
            tight=replace(self.limits,**{('binary_bytes' if kind=='binary' else 'source_bytes'):size-1})
            with self.assertRaisesRegex(ValueError,'signed_object_limit'):
                aio.verify_stream(stream,'https://fixture/input',digest,size,kind,tight)
            self.assertEqual(stream.opens,1)  # refuse before opening

    def test_actual_signed_source_bytes_through_online_adapter_and_retained(self):
        _,sources=self.signed_rows()
        row=next(r for r in sources if r['Package']=='linux')
        checksum,size,_=next(line.split() for line in row['Checksums-Sha256'].splitlines() if line.endswith('.dsc'))
        data=(FIXTURE/'linux-7.2.6.dsc').read_bytes()
        self.assertEqual((sha(data),len(data)),(checksum,int(size)))
        with response_worker(data):
            aio.verify_stream(aio.fetch,'https://fixture/source',checksum,int(size),'source-archive',self.limits)
        get=self.retained(data)
        # The same file succeeds beyond a scaled metadata cap via open(), while
        # the bounded metadata call still rejects it. No global cap increase.
        with self.assertRaisesRegex(ValueError,'download_limit'):get('https://fixture/source',limit=512)
        aio.verify_stream(get,'https://fixture/source',checksum,int(size),'source-archive',self.limits)

    def retained(self,data,checksum=None):
        (self.root/'blob').write_bytes(data)
        manifest=self.root/'inputs.json'
        manifest.write_text(json.dumps({'https://fixture/source':dict(path='blob',sha256=checksum or sha(data))}))
        return discover.retained_fetch(manifest)

    def test_stream_chunks_truncation_extra_bytes_hash_and_signed_shape(self):
        data=b'x'*8193;limits=replace(self.limits,source_bytes=len(data))
        for body in (data,data[:-1],data+b'y',data[:-1]+b'y'):
            stream=Streaming(body);out=io.BytesIO()
            with patch.object(aio,'CHUNK',257),patch.object(aio,'METADATA_BYTES',512):
                if body==data:
                    aio.verify_stream(stream,'https://fixture/source',sha(data),len(data),'source-archive',limits,out)
                    self.assertEqual(out.getvalue(),data)
                else:
                    with self.assertRaisesRegex(ValueError,'archive_hash_mismatch'):
                        aio.verify_stream(stream,'https://fixture/source',sha(data),len(data),'source-archive',limits,out)
            self.assertLessEqual(max(stream.reads),257)
            self.assertLessEqual(len(out.getvalue()),len(data))
        for checksum,size in (('z'*64,1),(sha(data),-1),(sha(data),True),(sha(data),'8193')):
            with self.assertRaisesRegex(ValueError,'signed_object_invalid'):
                aio.verify_stream(Streaming(data),'https://fixture/source',checksum,size,'source-archive',limits)

    def test_retained_never_substitutes_local_digest_for_signed_digest(self):
        good=b'original';wrong=b'changed!'
        with self.assertRaisesRegex(ValueError,'archive_hash_mismatch'):
            aio.verify_stream(self.retained(wrong),'https://fixture/source',sha(good),len(good),'source-archive',self.limits)
        with self.assertRaisesRegex(ValueError,'retained_input_changed'):
            aio.verify_stream(self.retained(good,'0'*64),'https://fixture/source',sha(good),len(good),'source-archive',self.limits)
        for wrong in (good[:-1],good+b'x'):
            with self.assertRaisesRegex(ValueError,'archive_hash_mismatch'):
                aio.verify_stream(self.retained(wrong),'https://fixture/source',sha(good),len(good),'source-archive',self.limits)
        with self.assertRaises(FileNotFoundError):
            aio.verify_stream(self.retained(good),'https://fixture/missing',sha(good),len(good),'source-archive',self.limits)

    def test_verified_source_cache_and_aggregate_budget(self):
        stream=Streaming(b'x'*100)
        payloads=aio.Payloads(stream,replace(self.limits,total_payload_bytes=150))
        payloads.source('https://fixture/a',sha(stream.data),100)
        payloads.source('https://fixture/alias',sha(stream.data),100)
        self.assertEqual((stream.opens,payloads.bytes,payloads.objects),(1,100,1))
        with self.assertRaisesRegex(ValueError,'total_payload_limit'):
            payloads.source('https://fixture/b','a'*64,100)
        with self.assertRaisesRegex(ValueError,'archive_hash_mismatch'):
            aio.Payloads(Streaming(b'y'),self.limits).source('https://fixture/a',sha(b'x'),1)

    def test_private_binary_snapshot_and_disk_failure_cleanup(self):
        data=deb([('./usr/lib/firmware/blob',b'x'*4097)])
        payloads=aio.Payloads(Streaming(data),self.limits)
        with payloads.binary('https://fixture/pkg',sha(data),len(data),self.root) as path:
            self.assertEqual(path.read_bytes(),data)
            self.assertEqual(aio.package_files(path),{'/usr/lib/firmware/blob':sha(b'x'*4097)})
        self.assertFalse(path.exists())
        with patch('shutil.disk_usage',return_value=type('Disk',(),{'free':len(data)-1})()):
            with self.assertRaisesRegex(ValueError,'payload_disk_space'):
                with payloads.binary('https://fixture/pkg',sha(data),len(data),self.root):self.fail('yielded')
        bad=aio.Payloads(Streaming(data[:-1]),self.limits)
        with self.assertRaisesRegex(ValueError,'archive_hash_mismatch'):
            with bad.binary('https://fixture/pkg',sha(data),len(data),self.root):self.fail('yielded')
        self.assertEqual(list(self.root.iterdir()),[])

    def inventory(self,members,limits=None,trailer=b''):
        stats={};result=aio.package_files(deb(members,trailer),limits=limits or self.limits,stats=stats)
        return result,stats

    def test_inventory_complete_multiple_files_without_whole_payload_read(self):
        members=[('./usr/lib/firmware/a',b'a'*5000),('./lib/firmware/b',b'b'*6000),('./usr/share/firmware/c',b'c'*7000),('./opt/ignored',b'd'*8000)]
        files,stats=self.inventory(members)
        self.assertEqual(files,{'/'+p.removeprefix('./'):sha(b) for p,b in members[:3]})
        self.assertEqual(stats['regular_bytes'],26000);self.assertEqual(stats['members'],4)
        self.assertGreater(stats['expanded_bytes'],26000)
        with self.assertRaisesRegex(ValueError,'expanded_package_limit'):
            self.inventory([('./opt/ignored',b'x'*16384)],replace(self.limits,expanded_package_bytes=8192))
        with self.assertRaisesRegex(ValueError,'package_member_limit'):
            self.inventory(members,replace(self.limits,member_bytes=4999))
        with self.assertRaisesRegex(ValueError,'package_member_limit'):
            self.inventory(members,replace(self.limits,package_members=3))
        with self.assertRaisesRegex(ValueError,'package_inventory_limit'):
            self.inventory(members,replace(self.limits,inventory_json_bytes=32))

    def test_archive_paths_duplicates_extensions_truncation_and_trailing_members(self):
        for name in ('./usr/../escape','./usr/'+('x'*4096)):
            with self.assertRaisesRegex(ValueError,'package_path'):self.inventory([(name,b'x')])
        with self.assertRaisesRegex(ValueError,'package_duplicate_path'):
            self.inventory([('./usr/a',b'x'),('usr/a',b'y')])
        header=tarfile.TarInfo('pax');header.type=tarfile.XHDTYPE;header.size=2*aio.CHUNK
        with self.assertRaisesRegex(ValueError,'package_archive_extension'):
            aio.package_files(deb_tar(header.tobuf()+b'\0'*(2*aio.CHUNK+10240)))
        header=tarfile.TarInfo('usr/a');header.size=16384
        with self.assertRaisesRegex(ValueError,'invalid_deb'):
            aio.package_files(deb_tar(header.tobuf()+b'x'*512))
        with self.assertRaisesRegex(ValueError,'invalid_deb'):
            self.inventory([('usr/a',b'a')],trailer=b'bad header'+b'X'*502)
        # A later archive after zero padding must not evade the inventory.
        second=io.BytesIO()
        with tarfile.open(fileobj=second,mode='w') as archive:
            member=tarfile.TarInfo('usr/b');member.size=1;archive.addfile(member,io.BytesIO(b'b'))
        files,_=self.inventory([('usr/a',b'a')],trailer=second.getvalue())
        self.assertEqual(files,{'/usr/a':sha(b'a'),'/usr/b':sha(b'b')})

    def test_stalled_decompressor_deadline_and_cleanup(self):
        original=subprocess.Popen;children=[]
        def stalled(*args,**kwargs):
            child=original([sys.executable,'-c','import time;time.sleep(30)'],**kwargs)
            children.append(child);return child
        with patch.object(aio.subprocess,'Popen',side_effect=stalled):
            with self.assertRaisesRegex(ValueError,'package_timeout'):
                aio.package_files(b'ignored',limits=replace(self.limits,package_seconds=1))
        self.assertIsNotNone(children[0].poll())

    def test_discovery_streams_signed_large_record_and_rejects_truncated_body(self):
        path=FIXTURE/'multipocket'
        config=json.loads((path/'config.json').read_text())
        config['resolve_metadata_only']=False
        for source in config['archives']:source['keyring']=str(path/source['keyring'])
        binary,_=self.signed_rows()
        row=next(r for r in binary if r['Package']=='linux-firmware')
        config['cohort_lock']['linux-firmware']=row['Version']
        retained=discover.retained_fetch(path/'inputs.json');payload=Streaming(b'')
        class Inputs:
            @contextmanager
            def open(self,url):
                if url.endswith('/'+row['Filename']):
                    with payload.open(url) as stream:yield stream
                else:
                    with retained.open(url) as stream:yield stream
        with self.assertRaisesRegex(ValueError,'archive_hash_mismatch'):
            discover.discover(config,{},self.root/'candidate.json',dt.datetime(2026,10,23,18,tzinfo=dt.timezone.utc),Inputs())
        self.assertEqual(payload.opens,1)
        self.assertFalse((self.root/'candidate.json').exists())
        self.assertEqual(list(self.root.glob('.verified-deb-*')),[])

    def test_metadata_expiry_during_streaming_never_emits_candidate(self):
        path=FIXTURE/'multipocket'
        config=json.loads((path/'config.json').read_text())
        for source in config['archives']:source['keyring']=str(path/source['keyring'])
        real_clock=aio.time.monotonic
        elapsed=[0]
        # The final freshness check precedes dependency validation, so advance
        # only after the last signed index has been read.
        real_index=discover.download_index
        def last_index(*args):
            rows=real_index(*args)
            if args[0].endswith('noble-security') and 'Sources' in args[1]:elapsed[0]=10*86400
            return rows
        with patch.object(discover,'download_index',side_effect=last_index),patch.object(discover.time,'monotonic',side_effect=lambda:real_clock()+elapsed[0]):
            with self.assertRaisesRegex(ValueError,'metadata_expired'):
                discover.discover(config,{},self.root/'candidate.json',dt.datetime(2026,10,23,18,tzinfo=dt.timezone.utc),discover.retained_fetch(path/'inputs.json'))
        self.assertFalse((self.root/'candidate.json').exists())

    def test_retention_after_hash_and_complete_inventory_only(self):
        # Structural workflow fixture: real signatures/indexes are checked first;
        # synthetic rows are injected afterward. Sources remain unavailable, so
        # this fixture never emits an admissible discovered/native candidate.
        path=FIXTURE/'multipocket'
        config=json.loads((path/'config.json').read_text())
        config['resolve_metadata_only']=False;config['retain_packages']=True
        config['cohort_lock']={name:'1' for name in config['cohort_lock'] if name.startswith('cosmic-')}
        for source in config['archives']:source['keyring']=str(path/source['keyring'])
        data=deb([('./usr/lib/firmware/a',b'a'*4097),('./lib/firmware/b',b'b'*8193)])
        retained=discover.retained_fetch(path/'inputs.json');payload=Streaming(data)
        class Inputs:
            @contextmanager
            def open(self,url):
                if url.endswith('/fixture.deb'):
                    with payload.open(url) as stream:yield stream
                else:
                    with retained.open(url) as stream:yield stream
        real=discover.download_index
        def rows(base,index,hashes,get):
            real(base,index,hashes,get)
            return [dict(Package=name,Version='1',Architecture='amd64',Filename='fixture.deb',SHA256=sha(data),Size=str(len(data)))
                    for name in config['cohort_lock']] if 'Packages' in index else []
        with patch.object(discover,'download_index',side_effect=rows):
            result=discover.discover(config,{},self.root/'candidate.json',dt.datetime(2026,10,23,18,tzinfo=dt.timezone.utc),Inputs())
        self.assertEqual(result['state'],'blocked')
        target=self.root/'packages'/(sha(data)+'.deb')
        self.assertEqual(target.read_bytes(),data)
        self.assertEqual(result['cohort']['packages']['cosmic-panel']['files'],
                         {'/usr/lib/firmware/a':sha(b'a'*4097),'/lib/firmware/b':sha(b'b'*8193)})
        self.assertEqual(result['resources']['capacity']['inventory_files'],10)
        target.unlink();payload.data=data[:-1]+b'X'
        with patch.object(discover,'download_index',side_effect=rows),patch.object(discover,'package_files') as inventory:
            with self.assertRaisesRegex(ValueError,'archive_hash_mismatch'):
                discover.discover(config,{},self.root/'candidate.json',dt.datetime(2026,10,23,18,tzinfo=dt.timezone.utc),Inputs())
            inventory.assert_not_called()
        self.assertFalse(target.exists())
        # Authenticated bytes with a malformed archive cannot be published either.
        data=deb([('./usr/../escape',b'x')]);payload.data=data
        with patch.object(discover,'download_index',side_effect=rows):
            with self.assertRaisesRegex(ValueError,'package_path'):
                discover.discover(config,{},self.root/'candidate.json',dt.datetime(2026,10,23,18,tzinfo=dt.timezone.utc),Inputs())
        self.assertEqual(list((self.root/'packages').iterdir()),[])

    def test_stream_memory_does_not_scale_with_payload_size(self):
        import tracemalloc
        size=8*aio.CHUNK+17
        expected=hashlib.sha256()
        for _ in range(8):expected.update(b'x'*aio.CHUNK)
        expected.update(b'x'*17)
        class Generated:
            @contextmanager
            def open(self,url):
                class Reader:
                    remaining=size
                    def read(self,n):
                        assert 0<n<=aio.CHUNK
                        value=b'x'*min(n,self.remaining);self.remaining-=len(value);return value
                yield Reader()
        tracemalloc.start()
        try:
            aio.verify_stream(Generated(),'https://fixture/stream',expected.hexdigest(),size,'source-archive',self.limits)
            _,peak=tracemalloc.get_traced_memory()
        finally:tracemalloc.stop()
        self.assertLess(peak,3*aio.CHUNK)
        print(json.dumps(dict(streamed_test_bytes=size,python_peak_bytes=peak,native_qualification=False)))

    def test_payload_deadline_cancels_worker_during_open(self):
        with response_worker(b'x'*100,delay=30):
            with self.assertRaisesRegex(ValueError,'payload_timeout'):
                aio.verify_stream(aio.fetch,'https://fixture/source',sha(b'x'*100),100,'source-archive',replace(self.limits,payload_seconds=1))

    def test_resource_config_cannot_relax_policy_and_http_is_rejected(self):
        for values in ({'unknown':1},{'source_bytes':0},{'source_bytes':True},{'source_bytes':2**60}):
            with self.assertRaisesRegex(ValueError,'resource_limits_invalid'):aio.Limits.configured(values)
        self.assertEqual(aio.Limits.configured({'source_bytes':123}).source_bytes,123)
        with self.assertRaisesRegex(ValueError,'https_required'):
            with aio.https_open('http://fixture/source'):self.fail('opened')
        with response_worker(b'x',final_url='http://fixture/source'):
            with self.assertRaisesRegex(ValueError,'insecure_redirect'):
                aio.verify_stream(aio.fetch,'https://fixture/source',sha(b'x'),1,'source-archive',self.limits)


class Capacity(unittest.TestCase):
    def test_exact_json_measurement_preserves_whole_identity(self):
        cohort=dict(distribution='pop',suite='noble',architecture='amd64',processes={},packages={'pkg':dict(version='1',sha256='a'*64,files={'/usr/é':'b'*64},source={'identity':'c'*64},url='https://fixture/p',provenance=['proof'])})
        original=discover.canonical(cohort)
        report=capacity.candidate_capacity(dict(cohort=cohort))
        self.assertEqual(report['packages'],1);self.assertEqual(report['inventory_files'],1)
        self.assertTrue(report['inventory_complete']);self.assertTrue(report['final_catalog_required'])
        self.assertEqual(capacity.encoded_size(cohort),len(discover.canonical(cohort)))
        self.assertEqual(discover.canonical(cohort),original)
        self.assertLess(report['identity_cohort_envelope_lower_bound_bytes'],report['full_cohort_envelope_bytes'])
        with tempfile.TemporaryDirectory() as work:
            path=Path(work)/'catalog.json'
            path.write_bytes(discover.canonical(dict(format='halcyon-islands-catalog',version=1,cohorts=[])))
            self.assertTrue(capacity.catalog_capacity(path)['schema_valid'])
            with path.open('wb') as f:f.truncate(capacity.CATALOG_BYTES+1)
            actual=capacity.catalog_capacity(path)
            self.assertFalse(actual['within_reader']);self.assertFalse(actual['schema_valid'])
            self.assertEqual(actual['error'],'not_bounded_record')

    def test_final_catalog_must_include_the_exact_candidate(self):
        from test_generation_contract import catalog_fixture
        catalog=catalog_fixture();row=catalog['cohorts'][0]
        candidate=dict(id=row['id'],cohort={k:row[k] for k in ('distribution','suite','architecture','packages','processes')})
        with tempfile.TemporaryDirectory() as work:
            path=Path(work)/'catalog.json';path.write_bytes(discover.canonical(catalog))
            report=capacity.catalog_capacity(path,candidate)
            self.assertTrue(report['schema_valid']);self.assertTrue(report['candidate_bound'])
            candidate['cohort']['packages']['cosmic-panel']['sha256']='f'*64
            self.assertFalse(capacity.catalog_capacity(path,candidate)['candidate_bound'])
            path.write_bytes(discover.canonical(dict(format='halcyon-islands-catalog',version=1,cohorts=[])))
            report=capacity.catalog_capacity(path,candidate)
            self.assertTrue(report['schema_valid']);self.assertFalse(report['candidate_bound'])

    def test_candidate_write_limit_is_atomic(self):
        with tempfile.TemporaryDirectory() as work:
            path=Path(work)/'candidate.json';path.write_bytes(b'previous')
            with patch.object(capacity,'CANDIDATE_BYTES',32):
                with self.assertRaisesRegex(ValueError,'candidate_record_limit'):
                    capacity.write_candidate(path,dict(value='x'*40))
            self.assertEqual(path.read_bytes(),b'previous')
            self.assertEqual(list(Path(work).iterdir()),[path])

if __name__=='__main__':unittest.main()

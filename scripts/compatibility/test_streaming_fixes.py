"""Actual HTTPResponse/decompressor regressions for review F1–F3. No network."""
from dataclasses import replace
import gzip
import hashlib
import io
import json
import os
from pathlib import Path
import tarfile
import tempfile
import threading
import time
import tracemalloc
import unittest
from unittest.mock import patch

import archive_io as aio
import http_transport
from test_large_inputs import deb_tar
from transport_test_support import response_worker, wire_worker


def digest(data):
    return hashlib.sha256(data).hexdigest()


def pax_record(key, value):
    tail=(' '+key+'='+value+'\n').encode()
    size=len(tail)+1
    while len(str(size))+len(tail)!=size:
        size=len(str(size))+len(tail)
    return str(size).encode()+tail


def header(name, body, kind=tarfile.REGTYPE):
    info=tarfile.TarInfo(name);info.size=len(body);info.type=kind
    return info.tobuf(format=tarfile.USTAR_FORMAT)+body+b'\0'*((-len(body))%512)


def compressed_deb(raw):
    # Build before measuring inventory memory. No extraction or installation.
    result=bytearray(b'!<arch>\n')
    for name,body in [('debian-binary',b'2.0\n'),('control.tar',b'\0'*10240),('data.tar.gz',gzip.compress(raw))]:
        result.extend(f'{name+"/":<16}{0:<12}{0:<6}{0:<6}{"100644":<8}{len(body):<10}`\n'.encode()+body+(b'\n' if len(body)%2 else b''))
    return bytes(result)


def extension_measurement(directory, count, kind):
    output=io.BytesIO()
    with tarfile.open(fileobj=output,mode='w',format=tarfile.PAX_FORMAT if kind=='pax' else tarfile.GNU_FORMAT) as archive:
        for number in range(count):
            member=tarfile.TarInfo(f'opt/outside/{number}')
            if kind=='pax':member.pax_headers={'comment':'x'*(256*1024)}
            else:member.type=tarfile.LNKTYPE;member.linkname='x'*(256*1024)
            archive.addfile(member,io.BytesIO(b''))
        # Prove dropping history still visits and hashes later regular files.
        member=tarfile.TarInfo('usr/after');member.size=5
        archive.addfile(member,io.BytesIO(b'after'))
    path=directory/f'{kind}-{count}.deb';path.write_bytes(compressed_deb(output.getvalue()));del output
    stats={};tracemalloc.start()
    try:
        files=aio.package_files(path,stats=stats)
        _,peak=tracemalloc.get_traced_memory()
    finally:tracemalloc.stop()
    return dict(kind=kind,members=count,python_peak_bytes=peak,files=files,stats=stats,package_bytes=path.stat().st_size)


class ArchiveExtensions(unittest.TestCase):
    def test_ordinary_pax_x_and_solaris_X_path_metadata_size(self):
        path='usr/'+('deep/'*40)+'file'
        for kind in (tarfile.XHDTYPE,tarfile.SOLARIS_XHDTYPE):
            payload=b''.join(pax_record(k,v) for k,v in [('path',path),('comment','ordinary metadata'),('uname','fixture'),('size','2')])
            # PAX size supersedes the following header's size, just as native tar.
            regular=tarfile.TarInfo('placeholder');regular.size=1
            raw=header('PaxHeader',payload,kind)+regular.tobuf()+b'xy'+b'\0'*510+header('lib/next',b'z')+b'\0'*1024
            with self.subTest(kind=kind):
                self.assertEqual(aio.package_files(deb_tar(raw)),{'/'+path:digest(b'xy'),'/lib/next':digest(b'z')})

    def test_pax_paths_remain_checked_and_bad_records_fail(self):
        for path in ('usr/../escape','usr/'+('x'*4096)):
            raw=header('PaxHeader',pax_record('path',path),tarfile.XHDTYPE)+header('placeholder',b'x')+b'\0'*1024
            with self.assertRaisesRegex(ValueError,'package_path'):aio.package_files(deb_tar(raw))
        raw=header('PaxHeader',b'0 path=x\n',tarfile.XHDTYPE)+header('usr/file',b'x')+b'\0'*1024
        with self.assertRaisesRegex(ValueError,'invalid_deb'):aio.package_files(deb_tar(raw))

    def test_extension_preallocation_chain_and_decoded_link_bounds(self):
        limits=replace(aio.Limits(),member_metadata_bytes=1024)
        raw=header('PaxHeader',pax_record('comment','x'*1100),tarfile.XHDTYPE)+header('usr/file',b'x')
        with self.assertRaisesRegex(ValueError,'package_archive_extension'):aio.package_files(deb_tar(raw),limits=limits)
        # Each individual extension fits; the recursive chain's sum does not.
        raw=header('longname',b'usr/'+b'x'*600+b'\0',tarfile.GNUTYPE_LONGNAME)*2+header('usr/file',b'x')
        with self.assertRaisesRegex(ValueError,'package_archive_extension'):aio.package_files(deb_tar(raw),limits=limits)
        raw=header('longname',b'usr/file\0',tarfile.GNUTYPE_LONGNAME)*17+header('usr/file',b'x')
        with self.assertRaisesRegex(ValueError,'package_archive_extension'):aio.package_files(deb_tar(raw))
        link=tarfile.TarInfo('opt/link');link.type=tarfile.LNKTYPE;link.linkname='x'*99;link.uname='u'*32;link.gname='g'*32
        with self.assertRaisesRegex(ValueError,'package_metadata_limit'):
            aio.package_files(deb_tar(link.tobuf()+b'\0'*1024),limits=replace(limits,member_metadata_bytes=128))

    def test_sparse_global_unknown_still_rejected_and_links_unfollowed(self):
        for kind in (tarfile.GNUTYPE_SPARSE,tarfile.XGLTYPE,b'z'):
            with self.subTest(kind=kind),self.assertRaisesRegex(ValueError,'package_archive_extension'):
                aio.package_files(deb_tar(header('unsupported',b'',kind)+b'\0'*1024))
        for sparse in ('GNU.sparse.map','GNU.sparse.major'):
            payload=pax_record(sparse,'1')+pax_record('GNU.sparse.minor','0')
            with self.assertRaises(ValueError):
                aio.package_files(deb_tar(header('PaxHeader',payload,tarfile.XHDTYPE)+header('usr/file',b'x')+b'\0'*1024))
        raw=header('usr/regular',b'x')
        for kind in (tarfile.LNKTYPE,tarfile.SYMTYPE):
            member=tarfile.TarInfo('usr/alias-'+kind.decode());member.type=kind;member.linkname='usr/regular'
            raw+=member.tobuf()
        self.assertEqual(aio.package_files(deb_tar(raw+b'\0'*1024)),{'/usr/regular':digest(b'x')})

    def test_many_longlink_and_pax_records_do_not_accumulate(self):
        with tempfile.TemporaryDirectory() as work:
            rows=[extension_measurement(Path(work),n,kind) for kind in ('pax','gnu_longlink') for n in (1,8,32)]
        for row in rows:
            self.assertEqual(row['files'],{'/usr/after':digest(b'after')})
            self.assertEqual(row['stats']['members'],row['members']+1)
            self.assertLess(row['python_peak_bytes'],5*1024**2)
        for group in (rows[:3],rows[3:]):
            self.assertLess(group[-1]['python_peak_bytes'],group[0]['python_peak_bytes']+2*1024**2)
        print(json.dumps(dict(extension_memory=rows,native_qualification=False)))


class TransportDeadlines(unittest.TestCase):
    def test_real_http_chunk_headers_and_trailers_strict_deadline_and_cleanup(self):
        fds=len(list(Path('/proc/self/fd').iterdir()));threads=threading.active_count();rows=[]
        for phase in ('chunk','headers','trailers'):
            started=time.monotonic()
            with self.subTest(phase=phase),wire_worker(phase,socket_timeout=30) as children:
                with self.assertRaisesRegex(ValueError,'^payload_timeout$'):
                    aio.verify_stream(aio.fetch,'https://fixture/source',digest(b'x'),1,'source-archive',replace(aio.Limits(),payload_seconds=1))
                self.assertEqual(len(children),1)
                self.assertIsNotNone(children[0].returncode)
                with self.assertRaises(ChildProcessError):os.waitpid(children[0].pid,os.WNOHANG)
            elapsed=time.monotonic()-started
            self.assertLess(elapsed,2.0)  # not 31s, not the 33s framing duration
            rows.append(dict(phase=phase,payload_seconds=1,socket_timeout=30,elapsed_seconds=elapsed))
        self.assertEqual(threading.active_count(),threads)
        self.assertLessEqual(len(list(Path('/proc/self/fd').iterdir())),fds)
        print(json.dumps(dict(transport_deadlines=rows,native_qualification=False)))

    def test_metadata_fetch_includes_headers_and_worker_open_is_cancellable(self):
        # Lower only the production default budget; keep actual HTTPResponse.
        with wire_worker('headers'):
            opener=lambda url:http_transport.open_https(url,deadline=time.monotonic()+1)
            started=time.monotonic()
            with patch.object(aio,'https_open',side_effect=opener),self.assertRaisesRegex(ValueError,'metadata_timeout'):
                aio.fetch('https://fixture/source',limit=10)
            self.assertLess(time.monotonic()-started,2)
        with response_worker(b'x',delay=30) as children:
            started=time.monotonic()
            with self.assertRaisesRegex(ValueError,'payload_timeout'):
                aio.verify_stream(aio.fetch,'https://fixture/source',digest(b'x'),1,'source-archive',replace(aio.Limits(),payload_seconds=1))
            self.assertLess(time.monotonic()-started,2);self.assertIsNotNone(children[0].returncode)

    def test_real_http_frames_complete_before_deadline(self):
        for phase in ('chunk','headers','trailers'):
            with self.subTest(phase=phase),wire_worker(phase,ticks=2,interval=.001) as children:
                aio.verify_stream(aio.fetch,'https://fixture/source',digest(b'x'),1,'source-archive',replace(aio.Limits(),payload_seconds=1))
                self.assertEqual(children[0].returncode,0)
        with wire_worker('chunk',ticks=2,interval=.001):
            with self.assertRaisesRegex(ValueError,'archive_hash_mismatch'):
                aio.verify_stream(aio.fetch,'https://fixture/source',digest(b'y'),1,'source-archive',aio.Limits())

    def test_body_success_mismatch_and_early_close_all_reap_worker(self):
        for body in (b'x',b'',b'y',b'xx'):
            with self.subTest(body=body),response_worker(body) as children:
                if body==b'x':aio.verify_stream(aio.fetch,'https://fixture/source',digest(b'x'),1,'source-archive',aio.Limits())
                else:
                    with self.assertRaisesRegex(ValueError,'archive_hash_mismatch'):
                        aio.verify_stream(aio.fetch,'https://fixture/source',digest(b'x'),1,'source-archive',aio.Limits())
                self.assertIsNotNone(children[0].returncode)
        with response_worker(b'x'*1024**2) as children:
            with aio.https_open('https://fixture/source') as stream:self.assertEqual(stream.read(1),b'x')
            self.assertIsNotNone(children[0].returncode)


if __name__=='__main__':unittest.main()

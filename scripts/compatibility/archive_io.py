"""Bounded streaming for authenticated APT payloads; never grants admission."""
from contextlib import contextmanager
from dataclasses import asdict, dataclass
import hashlib
import io
import os
from pathlib import Path
import re
import select
import shutil
import signal
import stat
import subprocess
import tarfile
import tempfile
import time
import urllib.parse
import urllib.request

CHUNK = 1024 * 1024
METADATA_BYTES = 512 * 1024 * 1024


@dataclass(frozen=True)
class Limits:
    # Separate policy ceilings, not substitutes for each signed exact size.
    binary_bytes: int = 4 * 1024**3
    source_bytes: int = 2 * 1024**3
    total_payload_bytes: int = 32 * 1024**3
    expanded_package_bytes: int = 8 * 1024**3
    member_bytes: int = 4 * 1024**3
    package_members: int = 250_000
    path_bytes: int = 4096
    inventory_json_bytes: int = 64 * 1024**2
    disk_reserve_bytes: int = 256 * 1024**2
    payload_seconds: int = 1800
    package_seconds: int = 900

    @classmethod
    def configured(cls, values):
        defaults = asdict(cls())
        if not isinstance(values, dict) or set(values) - defaults.keys():
            raise ValueError('resource_limits_invalid')
        # Jobs may tighten limits; larger policy ceilings require code review.
        if any(type(v) is not int or not 0 < v <= defaults[k] for k, v in values.items()):
            raise ValueError('resource_limits_invalid')
        return cls(**values)


def signed_object(checksum, size, kind, limits):
    if not isinstance(checksum, str) or not re.fullmatch('[0-9a-f]{64}', checksum) or type(size) is not int or size < 0:
        raise ValueError('signed_object_invalid')
    ceiling = {'binary': limits.binary_bytes, 'source-archive': limits.source_bytes, 'index': METADATA_BYTES}[kind]
    if size > ceiling:
        raise ValueError('signed_object_limit')


@contextmanager
def https_open(url):
    if urllib.parse.urlparse(url).scheme != 'https':
        raise ValueError('https_required')
    with urllib.request.urlopen(url, timeout=30) as stream:
        if urllib.parse.urlparse(stream.url).scheme != 'https':
            raise ValueError('insecure_redirect')
        yield stream


def read_chunk(stream, size):
    # HTTPResponse.read1 returns after one socket read, allowing the absolute
    # deadline to be checked even if the server trickles a long body.
    return getattr(stream, 'read1', stream.read)(size)


def bounded_read(stream, limit):
    output, count = io.BytesIO(), 0
    deadline = time.monotonic() + 300
    while True:
        if time.monotonic() >= deadline:
            raise ValueError('metadata_timeout')
        block = read_chunk(stream, min(CHUNK, limit - count + 1))
        if time.monotonic() >= deadline:
            raise ValueError('metadata_timeout')
        if not block:
            return output.getvalue()
        count += len(block)
        if count > limit:
            raise ValueError('download_limit')
        output.write(block)


def fetch(url, limit=METADATA_BYTES):
    with https_open(url) as stream:
        return bounded_read(stream, limit)


fetch.open = https_open


class RetainedFetch:
    def __init__(self, manifest_path):
        import json
        self.base = manifest_path.parent
        with manifest_path.open('rb') as stream:
            self.manifest = json.loads(bounded_read(stream, 16 * 1024**2))

    @contextmanager
    def open(self, url):
        if urllib.parse.urlparse(url).scheme != 'https':
            raise ValueError('https_required')
        if url not in self.manifest:
            raise FileNotFoundError(url)
        item = self.manifest[url]
        # The local map is a locator/integrity check, not an authentication root.
        # Signed expected size/hash are enforced independently by the caller.
        if not re.fullmatch('[0-9a-f]{64}', item['sha256']):
            raise ValueError('retained_input_changed')
        path = (self.base / item['path']).resolve()
        fd = os.open(path, os.O_RDONLY | os.O_NONBLOCK | os.O_NOFOLLOW)
        with os.fdopen(fd, 'rb') as stream:
            before = os.fstat(stream.fileno())
            if not stat.S_ISREG(before.st_mode):
                raise ValueError('retained_input_not_regular')
            reader = HashReader(stream)
            yield reader
            after = os.fstat(stream.fileno())
            if (before.st_size, before.st_mtime_ns, before.st_ctime_ns) != (after.st_size, after.st_mtime_ns, after.st_ctime_ns) or reader.count != before.st_size or reader.hasher.hexdigest() != item['sha256']:
                raise ValueError('retained_input_changed')

    def __call__(self, url, limit=METADATA_BYTES):
        with self.open(url) as stream:
            return bounded_read(stream, limit)


class HashReader:
    def __init__(self, stream):
        self.stream, self.count, self.hasher = stream, 0, hashlib.sha256()

    def read(self, size):
        block = self.stream.read(size)
        self.count += len(block)
        self.hasher.update(block)
        return block


@contextmanager
def input_stream(get, url):
    # Byte-returning callbacks remain a small-fixture API. Production online and
    # retained adapters always expose open(), so payloads never use read_bytes.
    if hasattr(get, 'open'):
        with get.open(url) as stream:
            yield stream
    else:
        with io.BytesIO(get(url)) as stream:
            yield stream


def verify_stream(get, url, checksum, size, kind, limits, output=None):
    signed_object(checksum, size, kind, limits)
    if size > METADATA_BYTES and not hasattr(get, 'open'):
        raise ValueError('streaming_transport_required')
    deadline = time.monotonic() + limits.payload_seconds
    count, hasher = 0, hashlib.sha256()
    with input_stream(get, url) as stream:
        while True:
            if time.monotonic() >= deadline:
                raise ValueError('payload_timeout')
            block = read_chunk(stream, min(CHUNK, size - count + 1))
            if time.monotonic() >= deadline:
                raise ValueError('payload_timeout')
            if not block:
                break
            count += len(block)
            if count > size:
                raise ValueError('archive_hash_mismatch')
            hasher.update(block)
            if output is not None:
                output.write(block)
    if count != size or hasher.hexdigest() != checksum:
        raise ValueError('archive_hash_mismatch')


class Payloads:
    """Run-local verified source reuse and signed-byte/work accounting."""
    def __init__(self, get, limits):
        self.get, self.limits = get, limits
        self.sources = set()
        self.bytes = self.objects = self.inventory_bytes = 0

    def reserve(self, checksum, size, kind):
        signed_object(checksum, size, kind, self.limits)
        if self.bytes + size > self.limits.total_payload_bytes:
            raise ValueError('total_payload_limit')
        self.bytes += size
        self.objects += 1

    def source(self, url, checksum, size):
        signed_object(checksum, size, 'source-archive', self.limits)
        if (checksum, size) in self.sources:
            return
        self.reserve(checksum, size, 'source-archive')
        verify_stream(self.get, url, checksum, size, 'source-archive', self.limits)
        self.sources.add((checksum, size))

    @contextmanager
    def binary(self, url, checksum, size, workspace):
        self.reserve(checksum, size, 'binary')
        if shutil.disk_usage(workspace).free < size + self.limits.disk_reserve_bytes:
            raise ValueError('payload_disk_space')
        with tempfile.TemporaryDirectory(prefix='.verified-deb-', dir=workspace) as work:
            path = Path(work) / 'input.deb'
            with path.open('xb') as output:
                verify_stream(self.get, url, checksum, size, 'binary', self.limits, output)
                output.flush()
                os.fsync(output.fileno())
            yield path

    def account_inventory(self, files):
        import json
        self.inventory_bytes += sum(len(s.encode('utf-8')) for s in json.JSONEncoder(sort_keys=True, separators=(',', ':')).iterencode(files))
        if self.inventory_bytes > self.limits.inventory_json_bytes:
            raise ValueError('total_inventory_limit')


class BoundedTarInfo(tarfile.TarInfo):
    limits = Limits()

    @classmethod
    def fromtarfile(cls, archive):
        try:
            return super().fromtarfile(archive)
        except (tarfile.InvalidHeaderError, tarfile.TruncatedHeaderError) as error:
            # ignore_zeros must ONLY ignore zero padding, never a damaged header.
            raise ValueError('invalid_deb') from error

    def _proc_member(self, archive):
        # This hook runs before tarfile allocates extension contents (including
        # Python versions whose fromtarfile bypasses the public frombuf hook).
        count = getattr(archive, '_bounded_headers', 0) + 1
        archive._bounded_headers = count
        depth = getattr(archive, '_bounded_depth', 0) + 1
        archive._bounded_depth = depth
        if count > self.limits.package_members or self.size < 0 or self.size > self.limits.member_bytes:
            raise ValueError('package_member_limit')
        if depth > 16 or self.type in (tarfile.XGLTYPE, tarfile.GNUTYPE_SPARSE) or self.type not in tarfile.SUPPORTED_TYPES:
            raise ValueError('package_archive_extension')
        if self.type in (tarfile.XHDTYPE, tarfile.SOLARIS_XHDTYPE, tarfile.GNUTYPE_LONGNAME, tarfile.GNUTYPE_LONGLINK) and self.size > CHUNK:
            raise ValueError('package_archive_extension')
        try:
            return super()._proc_member(archive)
        finally:
            archive._bounded_depth -= 1

    def _proc_gnusparse_00(self, *args):
        raise ValueError('package_archive_extension')

    _proc_gnusparse_01 = _proc_gnusparse_00
    _proc_gnusparse_10 = _proc_gnusparse_00


class TarReader:
    """Count ALL expanded bytes, including ignored paths/headers/trailers.

    select + os.read prevents a stuck decompressor from trapping tarfile.read.
    """
    def __init__(self, pipe, limits):
        self.pipe, self.limits = pipe, limits
        self.deadline = time.monotonic() + limits.package_seconds
        self.count = 0

    def read(self, size):
        if size == 0:
            return b''
        remaining = self.deadline - time.monotonic()
        if remaining <= 0 or not select.select([self.pipe], [], [], remaining)[0]:
            raise ValueError('package_timeout')
        data = os.read(self.pipe.fileno(), min(size, CHUNK, self.limits.expanded_package_bytes - self.count + 1))
        self.count += len(data)
        if self.count > self.limits.expanded_package_bytes:
            raise ValueError('expanded_package_limit')
        return data


def package_files(package, *, limits=None, stats=None):
    """Stream all regular /usr and /lib members of a verified .deb snapshot.

    No extraction or execution of package content. Bytes accepted for existing
    small qualification fixtures; discovery supplies a private verified path.
    """
    limits = limits or Limits()
    if isinstance(package, bytes):
        with tempfile.TemporaryDirectory() as work:
            path = Path(work) / 'input.deb'
            path.write_bytes(package)
            return package_files(path, limits=limits, stats=stats)
    process = subprocess.Popen(['dpkg-deb', '--fsys-tarfile', str(package)], stdout=subprocess.PIPE,
                               stderr=subprocess.DEVNULL, start_new_session=True)
    reader = TarReader(process.stdout, limits)
    files, seen, count, inventory_bytes, regular_bytes, path_bytes, max_member = {}, set(), 0, 2, 0, 0, 0
    try:
        # ignore_zeros scans concatenated/trailing blocks rather than silently
        # ignoring payload after the first end marker. The reader bounds all IO.
        policy = type('PolicyTarInfo', (BoundedTarInfo,), {'limits': limits})
        with tarfile.open(fileobj=reader, mode='r|', ignore_zeros=True, tarinfo=policy) as archive:
            for member in archive:
                count += 1
                if count > limits.package_members:
                    raise ValueError('package_member_limit')
                path = '/' + member.name.removeprefix('./').lstrip('/')
                if '..' in Path(path).parts or '\x00' in path or len(path.encode('utf-8')) > limits.path_bytes:
                    raise ValueError('package_path')
                path = str(Path(path))
                if path in seen:
                    raise ValueError('package_duplicate_path')
                seen.add(path)
                path_bytes += len(path.encode('utf-8'))
                if path_bytes > limits.inventory_json_bytes:
                    raise ValueError('package_inventory_limit')
                if member.size < 0 or member.size > limits.member_bytes or member.issparse():
                    raise ValueError('package_member_limit')
                if member.isfile():
                    regular_bytes += member.size
                    max_member = max(max_member, member.size)
                    # Drain/hash EVERY regular file, even outside the runtime
                    # inventory. tarfile must not silently skip corrupt content.
                    stream = archive.extractfile(member)
                    hasher, length = hashlib.sha256(), 0
                    while block := stream.read(CHUNK):
                        length += len(block)
                        hasher.update(block)
                    if length != member.size:
                        raise ValueError('invalid_deb')
                    if path.startswith(('/usr/', '/lib/')):
                        import json
                        inventory_bytes += len(json.dumps(path, ensure_ascii=True).encode()) + 68
                        if inventory_bytes > limits.inventory_json_bytes:
                            raise ValueError('package_inventory_limit')
                        files[path] = hasher.hexdigest()
        # Consume remaining pipe data and check child completion even when the
        # tar parser has reached EOF. All bytes remain under the same deadline.
        while reader.read(CHUNK):
            pass
        if process.wait(timeout=max(.001, reader.deadline - time.monotonic())):
            raise ValueError('invalid_deb')
        if stats is not None:
            stats.update(expanded_bytes=reader.count, regular_bytes=regular_bytes,
                         members=count, inventory_files=len(files), inventory_json_bytes=inventory_bytes,
                         max_regular_file_bytes=max_member)
        return files
    except (tarfile.TarError, EOFError) as error:
        raise ValueError('invalid_deb') from error
    finally:
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        process.wait()
        process.stdout.close()

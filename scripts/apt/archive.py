"""Local immutable archive primitives. No network, installation or key generation."""
import email.utils
import gzip
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import time
import xml.etree.ElementTree as ET

PACKAGES = ('cosmic-ext-halcyon', 'cosmic-ext-halcyon-islands',
            'halcyon-archive-keyring', 'halcyon-update-source')
SUITE = Path('dists/noble')
APP_ID = 'io.github.profilaz3.Halcyon'


def canonical(value):
    return (json.dumps(value, sort_keys=True, separators=(',', ':')) + '\n').encode()


def sha(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def inventory(root, exclude=()):
    result = {}
    for p in sorted(root.rglob('*')):
        if p.is_symlink() or not (p.is_dir() or p.is_file()):
            raise ValueError('nonregular_archive_path')
        if p.is_file() and p.relative_to(root).as_posix() not in exclude:
            result[p.relative_to(root).as_posix()] = sha(p)
    return result


def verify_tree(root, expected):
    descriptor = root / 'archive.json'
    if sha(descriptor) != expected:
        raise ValueError('archive_review_digest_mismatch')
    data = json.loads(descriptor.read_bytes())
    if data.get('format') != 'halcyon-apt-stage' or data.get('version') != 1:
        raise ValueError('archive_schema')
    excluded = ('archive.json', 'archive.json.asc', 'signed.json', str(SUITE/'InRelease'), str(SUITE/'Release.gpg'))
    if inventory(root, excluded) != data['files']:
        raise ValueError('archive_bytes_changed')
    return data


def run(args, **kwargs):
    return subprocess.run(args, check=True, stdout=subprocess.PIPE,
                          stderr=subprocess.PIPE, timeout=120, **kwargs).stdout


def control(path):
    output = run(['dpkg-deb', '-f', str(path.resolve())]).decode()
    fields = dict(line.split(': ', 1) for line in output.splitlines() if ': ' in line and not line.startswith(' '))
    name, version, arch = (fields[k] for k in ('Package', 'Version', 'Architecture'))
    if name not in PACKAGES or arch not in ('amd64', 'all'):
        raise ValueError('package_not_allowlisted')
    run(['dpkg', '--validate-version', version])
    return dict(package=name, version=version, architecture=arch, sha256=sha(path))


def compare(a, op, b):
    result = subprocess.run(['dpkg', '--compare-versions', a, op, b], capture_output=True, timeout=5)
    if result.returncode not in (0, 1):
        raise ValueError('invalid_debian_version')
    return result.returncode == 0


def write(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data)


def copy_file(source, destination):
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(source, destination)


def sync_directory(path):
    descriptor = os.open(path, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def sync_tree(root):
    for path in root.rglob('*'):
        if path.is_file():
            with path.open('rb') as stream:
                os.fsync(stream.fileno())
    for path in sorted((p for p in root.rglob('*') if p.is_dir()), key=lambda p: len(p.parts), reverse=True):
        sync_directory(path)
    sync_directory(root)


def appstream(package, root):
    """Use metainfo/icons from the exact final main package, never the checkout."""
    # Keep large package payloads out of memory, and never extract arbitrary
    # members (symlinks, traversal names or executable content) to the filesystem.
    with tempfile.TemporaryFile() as payload:
        subprocess.run(['dpkg-deb', '--fsys-tarfile', str(package.resolve())],
                       stdout=payload, stderr=subprocess.PIPE, check=True, timeout=120)
        payload.seek(0)
        with tarfile.open(fileobj=payload) as archive:
            return appstream_members(archive, root)


def appstream_members(archive, root):
    def read(name):
        member = archive.getmember('./' + name)
        if not member.isfile() or member.size > 2 * 1024 * 1024:
            raise ValueError('invalid_appstream_member')
        return archive.extractfile(member).read()
    component = ET.fromstring(read(f'usr/share/metainfo/{APP_ID}.metainfo.xml'))
    if component.tag != 'component' or component.findtext('id') != APP_ID:
        raise ValueError('appstream_wrong_identity')
    for element in list(component.findall('pkgname')) + list(component.findall('icon')):
        component.remove(element)
    ET.SubElement(component, 'pkgname').text = PACKAGES[0]
    collection = ET.Element('components', {'version': '1.0', 'origin': 'Halcyon'})
    for size in (64, 128):
        ET.SubElement(component, 'icon', {'type': 'cached', 'width': str(size), 'height': str(size)}).text = APP_ID + '.png'
        data = read(f'usr/share/icons/hicolor/{size}x{size}/apps/{APP_ID}.png')
        buf = io.BytesIO()
        with tarfile.open(fileobj=buf, mode='w', format=tarfile.USTAR_FORMAT) as icons:
            entry = tarfile.TarInfo(APP_ID + '.png')
            entry.size = len(data)
            entry.mode = 0o644
            icons.addfile(entry, io.BytesIO(data))
        write(root / SUITE / f'main/dep11/icons-{size}x{size}.tar.gz', gzip.compress(buf.getvalue(), mtime=0))
    collection.append(component)
    xml = root / SUITE / 'main/dep11/Components-amd64.xml'
    write(xml, ET.tostring(collection, encoding='utf-8', xml_declaration=True))
    yaml = xml.with_suffix('.yml')
    run(['appstreamcli', 'convert', '--format=yaml', str(xml), str(yaml)])
    for path in (xml, yaml):
        write(path.with_suffix(path.suffix + '.gz'), gzip.compress(path.read_bytes(), mtime=0))

def stage(packages, retained, out, epoch, previous=None, previous_sha=None, review=None):
    """Low-level fixture-capable builder. Production CLI always calls admission first."""
    if out.exists() or out.is_symlink():
        raise ValueError('immutable_output_exists')
    old = verify_tree(previous, previous_sha) if previous else None
    if old and epoch <= old['epoch']:
        raise ValueError('metadata_date_must_advance')
    out.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='.apt-stage-', dir=out.parent) as work:
        root = Path(work) / 'archive'
        root.mkdir()
        if previous:
            # Retain every payload, source/evidence and by-hash object. Retired
            # canonical indexes/Release signatures are regenerated below.
            for name in old['files']:
                if name.startswith(('pool/', 'releases/')) or '/by-hash/SHA256/' in name:
                    copy_file(previous/name, root/name)
        records = list(old['packages']) if old else []
        for package in packages:
            record = control(package)
            for existing in records:
                if (record['package'] == existing['package'] and record['architecture'] == existing['architecture']
                        and compare(record['version'], 'eq', existing['version'])
                        and record['sha256'] != existing['sha256']):
                    raise ValueError('immutable_version_collision')
            record['path'] = f'pool/main/{record["package"]}/{record["sha256"]}.deb'
            copy_file(package, root/record['path'])
            if sha(root/record['path']) != record['sha256']:
                raise ValueError('package_changed_during_staging')
            if record not in records:
                records.append(record)
        if {r['package'] for r in records} != set(PACKAGES):
            raise ValueError('four_enrollment_and_app_packages_required')
        for name, data in retained.items():
            rel = Path(name)
            if rel.is_absolute() or '..' in rel.parts or not name.startswith('releases/'):
                raise ValueError('unsafe_retained_path')
            target = root / name
            digest = sha(data) if isinstance(data, Path) else hashlib.sha256(data).hexdigest()
            if target.exists() and sha(target) != digest:
                raise ValueError('immutable_evidence_collision')
            if isinstance(data, Path):
                copy_file(data, target)
            else:
                write(target, data)
            if sha(target) != digest:
                raise ValueError('evidence_changed_during_staging')
        index = root / SUITE / 'main/binary-amd64/Packages'
        write(index, run(['apt-ftparchive', 'packages', 'pool'], cwd=root))
        write(index.with_suffix('.gz'), gzip.compress(index.read_bytes(), mtime=0))
        mains = [r for r in records if r['package'] == PACKAGES[0]]
        newest = mains[0]
        for record in mains[1:]:
            if compare(record['version'], 'gt', newest['version']):
                newest = record
        appstream(root / newest['path'], root)
        indexes = sorted(p for p in (root / SUITE).rglob('*') if p.is_file() and '/by-hash/' not in str(p))
        hashes = []
        for path in indexes:
            digest = sha(path)
            write(path.parent/'by-hash/SHA256'/digest, path.read_bytes())
            hashes.append(f' {digest} {path.stat().st_size:16} {path.relative_to(root / SUITE)}\n')
        date = email.utils.formatdate(epoch, usegmt=True)
        expires = email.utils.formatdate(epoch + 7*86400, usegmt=True)
        release = ('Origin: Halcyon\nLabel: Halcyon\nSuite: noble\nCodename: noble\n'
                   f'Date: {date}\nValid-Until: {expires}\nArchitectures: amd64\n'
                   'Components: main\nAcquire-By-Hash: yes\n'
                   'Description: Halcyon explicitly enrolled system updates\nSHA256:\n' + ''.join(hashes))
        write(root / SUITE / 'Release', release.encode())
        descriptor = dict(format='halcyon-apt-stage', version=1, epoch=epoch,
                          previous=previous_sha, review=review,
                          packages=sorted(records, key=lambda r: (r['package'], r['version'], r['sha256'])),
                          files=inventory(root))
        write(root/'archive.json', canonical(descriptor))
        sync_tree(root)
        os.rename(root, out)
        sync_directory(out.parent)
    return sha(out/'archive.json')


def verify_signed(root, expected, keyring):
    data = verify_tree(root, expected)
    descriptor = json.loads((root/'signed.json').read_bytes())
    signatures = {str(SUITE/name): sha(root/SUITE/name) for name in ('InRelease', 'Release.gpg')}
    signatures['archive.json.asc'] = sha(root/'archive.json.asc')
    if descriptor.get('archive_sha256') != expected or descriptor.get('signatures') != signatures:
        raise ValueError('signature_inventory_mismatch')
    with tempfile.TemporaryDirectory(prefix='halcyon-gpgv-') as home:
        common = ['gpgv', '--homedir', home, '--keyring', str(keyring.resolve()), '--status-fd', '1']
        clear = Path(home)/'clear'
        statuses = [run([*common, '--output', str(clear), str(root/SUITE/'InRelease')])]
        if clear.read_bytes() != (root/SUITE/'Release').read_bytes():
            raise ValueError('inrelease_payload_mismatch')
        statuses.append(run([*common, str(root/SUITE/'Release.gpg'), str(root/SUITE/'Release')]))
        statuses.append(run([*common, str(root/'archive.json.asc'), str(root/'archive.json')]))
        for status in statuses:
            valid = [line.split()[2] for line in status.decode().splitlines() if line.startswith('[GNUPG:] VALIDSIG ')]
            if valid != [descriptor['signing_subkey']]:
                raise ValueError('unexpected_archive_signer')
    signing_deadline(keyring, descriptor['signing_subkey'])
    return data


def signing_deadline(keyring, subkey):
    """Require a live, expiring signing subkey and live certification key."""
    with tempfile.TemporaryDirectory(prefix='halcyon-key-status-') as home:
        lines = run(['gpg', '--batch', '--homedir', home, '--with-colons', '--show-keys',
                     str(keyring.resolve())]).decode().splitlines()
    primary = current = None
    for line in lines:
        fields = line.split(':')
        if fields[0] == 'pub':
            primary = fields
        if fields[0] in ('pub', 'sub'):
            current = fields
        if fields[0] == 'fpr' and fields[9] == subkey:
            if not primary or current[0] != 'sub' or 's' not in current[11]:
                break
            if any(row[1] in ('r', 'e', 'd') or not row[6] or int(row[6]) <= time.time()
                   for row in (primary, current)):
                raise ValueError('signing_key_expired_revoked_or_unbounded')
            return min(int(row[6]) for row in (primary, current))
    raise ValueError('signing_subkey_not_in_trust_keyring')


def sign(root, expected, out, home, subkey, keyring):
    verify_tree(root, expected)
    if out.exists() or out.is_symlink() or (root/'signed.json').exists():
        raise ValueError('fresh_signing_output_required')
    # Require an explicitly selected signing subkey, not the certification key.
    listing = run(['gpg', '--batch', '--homedir', str(home), '--with-colons', '--list-secret-keys', subkey]).decode()
    previous = None
    found = False
    for line in listing.splitlines():
        fields = line.split(':')
        if fields[0] == 'fpr' and fields[9] == subkey and previous and previous[0] == 'ssb' and 's' in previous[11]:
            found = True
        if fields[0] in ('sec', 'ssb'):
            previous = fields
    if not found:
        raise ValueError('explicit_signing_subkey_required')
    signing_deadline(keyring, subkey)
    out.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='.apt-sign-', dir=out.parent) as work:
        staged = Path(work)/'archive'
        shutil.copytree(root, staged)
        release = staged/SUITE/'Release'
        common = ['gpg', '--batch', '--homedir', str(home), '--local-user', subkey+'!', '--digest-algo', 'SHA256']
        run([*common, '--output', str(release.with_name('InRelease')), '--clearsign', str(release)])
        run([*common, '--output', str(release.with_name('Release.gpg')), '--detach-sign', str(release)])
        run([*common, '--armor', '--output', str(staged/'archive.json.asc'), '--detach-sign', str(staged/'archive.json')])
        write(staged/'signed.json', canonical(dict(archive_sha256=expected, signing_subkey=subkey,
            signatures={**{str(SUITE/name): sha(staged/SUITE/name) for name in ('InRelease', 'Release.gpg')},
                        'archive.json.asc': sha(staged/'archive.json.asc')})))
        verify_signed(staged, expected, keyring)
        sync_tree(staged)
        os.rename(staged, out)
        sync_directory(out.parent)


def promote(root, expected, keyring, deployment, previous):
    """Local atomic directory-pointer promotion only. Pages deployment is external."""
    import fcntl
    verify_signed(root, expected, keyring)
    deployment.mkdir(parents=True, exist_ok=True)
    with (deployment/'.promotion.lock').open('a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        current = deployment/'current'
        actual = sha(current/'archive.json') if current.exists() else None
        if actual != previous:
            raise ValueError('deployment_predecessor_changed')
        data = json.loads((root/'archive.json').read_bytes())
        if data['previous'] != previous:
            raise ValueError('stage_did_not_retain_predecessor')
        generations = deployment/'generations'
        generations.mkdir(exist_ok=True)
        target = generations/expected
        if target.exists():
            verify_signed(target, expected, keyring)
        else:
            with tempfile.TemporaryDirectory(prefix='.promote-', dir=generations) as work:
                copy = Path(work)/'archive'
                shutil.copytree(root, copy)
                verify_signed(copy, expected, keyring)
                sync_tree(copy)
                os.rename(copy, target)
                sync_directory(generations)
        pointer = deployment/'.next'
        if pointer.is_symlink():
            pointer.unlink()
        pointer.symlink_to(Path('generations')/expected)
        os.replace(pointer, current)
        sync_directory(deployment)

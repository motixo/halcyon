#!/usr/bin/env python3
"""Bounded read-only signed APT discovery; never edits the shipped catalog."""
import argparse
import datetime as dt
import email.utils
import gzip
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import tarfile
import tempfile
import urllib.parse
import urllib.request

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'companion'))
from bundle_contract import canonical, identity

MAX_BYTES = 512 * 1024 * 1024


def sha(data):
    return hashlib.sha256(data).hexdigest()


def paragraphs(data):
    result, row, key = [], {}, None
    for line in data.decode('utf-8').splitlines() + ['']:
        if not line:
            if row:
                result.append(row)
            row, key = {}, None
        elif line[0].isspace():
            if key is None:
                raise ValueError('invalid_control_continuation')
            row[key] += '\n' + line[1:]
        else:
            key, value = line.split(':', 1)
            if key in row:
                raise ValueError('duplicate_control_field')
            row[key] = value.strip()
    return result


def fetch(url, limit=MAX_BYTES):
    if urllib.parse.urlparse(url).scheme != 'https':
        raise ValueError('https_required')
    with urllib.request.urlopen(url, timeout=30) as stream:
        if urllib.parse.urlparse(stream.url).scheme != 'https':
            raise ValueError('insecure_redirect')
        value = stream.read(limit + 1)
    if len(value) > limit:
        raise ValueError('download_limit')
    return value


def child_url(base, path):
    if path.startswith('/') or '..' in Path(path).parts or urllib.parse.urlparse(path).scheme or '?' in path or '#' in path:
        raise ValueError('archive_path')
    return base.rstrip('/') + '/' + path


def verify_release(signed, keyring, fingerprint, now, max_age_days=7):
    with tempfile.TemporaryDirectory() as work:
        work = Path(work)
        source = work / 'InRelease'; source.write_bytes(signed)
        output = work / 'Release'
        result = subprocess.run(['gpgv', '--homedir', str(work), '--keyring', str(keyring.resolve()),
                                 '--status-fd', '1', '--output', str(output), str(source)],
                                capture_output=True, timeout=15, check=False)
        valid = [line.split() for line in result.stdout.decode().splitlines() if line.startswith('[GNUPG:] VALIDSIG ')]
        rejected = any(tag in result.stdout for tag in (b'EXPKEYSIG', b'EXPSIG', b'REVKEYSIG', b'KEYEXPIRED', b'SIGEXPIRED'))
        if result.returncode or rejected or not any(v[2] == fingerprint or v[-1] == fingerprint for v in valid):
            raise ValueError('signature_invalid')
        release = paragraphs(output.read_bytes())[0]
    date = email.utils.parsedate_to_datetime(release['Date'])
    if date.tzinfo is None or date > now + dt.timedelta(minutes=10):
        raise ValueError('release_date_invalid')
    # Pop Release currently omits Valid-Until. A pinned, bounded signed-Date
    # freshness policy is stricter than accepting unbounded old metadata.
    expires = date + dt.timedelta(days=min(max_age_days, 7))
    if release.get('Valid-Until'):
        expires = min(expires, email.utils.parsedate_to_datetime(release['Valid-Until']))
    if now >= expires:
        raise ValueError('metadata_expired')
    hashes = {}
    for line in release['SHA256'].splitlines():
        if line.strip():
            checksum, size, path = line.split()
            if not re.fullmatch('[0-9a-f]{64}', checksum) or path in hashes:
                raise ValueError('release_hash_table')
            hashes[path] = (checksum, int(size))
    return release, hashes, expires.isoformat()


def checked(data, checksum, size):
    if len(data) != size or sha(data) != checksum:
        raise ValueError('archive_hash_mismatch')
    return data


def download_index(base, path, hashes, get=fetch):
    checksum, size = hashes[path]
    compressed = checked(get(child_url(base, path)), checksum, size)
    if path.endswith('.gz'):
        import io
        with gzip.GzipFile(fileobj=io.BytesIO(compressed)) as stream:
            data = stream.read(MAX_BYTES + 1)
        if len(data) > MAX_BYTES:
            raise ValueError('expanded_index_limit')
        return paragraphs(data)
    return paragraphs(compressed)


def package_files(data):
    """Inventory runtime files from verified deb, without extracting/executing it."""
    with tempfile.TemporaryDirectory() as work:
        package = Path(work) / 'input.deb'; package.write_bytes(data)
        process = subprocess.Popen(['dpkg-deb', '--fsys-tarfile', str(package)], stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
        files = {}
        try:
            with tarfile.open(fileobj=process.stdout, mode='r|') as archive:
                total = 0
                for member in archive:
                    path = '/' + member.name.removeprefix('./').lstrip('/')
                    if '..' in Path(path).parts:
                        raise ValueError('package_path')
                    # Configuration is mutable. Bind all regular shipped runtime
                    # bytes in /usr and /lib, including applets, data and libraries.
                    if member.isfile() and path.startswith(('/usr/', '/lib/')):
                        total += member.size
                        if total > 2 * MAX_BYTES:
                            raise ValueError('expanded_package_limit')
                        stream = archive.extractfile(member)
                        files[path] = hashlib.file_digest(stream, 'sha256').hexdigest()
            if process.wait(timeout=10):
                raise ValueError('invalid_deb')
        finally:
            if process.poll() is None:
                process.kill(); process.wait()
            process.stdout.close()
    return files


def dependency_gaps(selected):
    """Distribution parser/version semantics, including alternatives/providers."""
    import apt_pkg
    apt_pkg.init_system()
    provided = {}
    for name, item in selected.items():
        provided.setdefault(name, []).append(item['version'])
        for alternatives in apt_pkg.parse_depends(item.get('provides', '')):
            for virtual, version, _ in alternatives:
                provided.setdefault(virtual, []).append(version)
    gaps = []
    for name, item in selected.items():
        for field in ('depends', 'pre_depends'):
            for alternatives in apt_pkg.parse_depends(item.get(field, '')):
                if not any(any(not op or (v and apt_pkg.check_dep(v, op, version))
                               for v in provided.get(dep.removesuffix(':any').removesuffix(':native'), []))
                           for dep, version, op in alternatives):
                    gaps.append('dependency_unresolved:' + name + ':' + alternatives[0][0])
    return sorted(set(gaps))


def discover(config, previous, out, now=None, get=fetch):
    now = now or dt.datetime.now(dt.timezone.utc)
    releases, packages, sources = [], {}, {}
    for source in config['archives']:
        key = Path(source['keyring'])
        if sha(key.read_bytes()) != source['keyring_sha256']:
            raise ValueError('trust_root_changed')
        base = source['url'].rstrip('/')
        dist = child_url(base, 'dists/' + config['suite'])
        signed = get(child_url(dist, 'InRelease'))
        release, hashes, expires = verify_release(signed, key, source['fingerprint'], now)
        if release['Codename'] != config['suite'] or config['architecture'] not in release['Architectures'].split() or release['Origin'] != source['origin']:
            raise ValueError('archive_scope_mismatch')
        releases.append(dict(url=dist, sha256=sha(signed), expires=expires, fingerprint=source['fingerprint']))
        for component in source['components']:
            for row in download_index(dist, component + '/binary-' + config['architecture'] + '/Packages.gz', hashes, get):
                if row['Architecture'] in (config['architecture'], 'all'):
                    packages[(row['Package'], row['Version'])] = (row, base)
            path = component + '/source/Sources.gz'
            if path in hashes:
                for row in download_index(dist, path, hashes, get):
                    sources[(row['Package'], row['Version'])] = (row, base)
    # A complete image's exact lock is authoritative for dependency resolution.
    # Three latest versions alone never constitute a qualification environment.
    lock = config.get('cohort_lock', {})
    reasons = []
    if not lock or not {'cosmic-panel','cosmic-comp','cosmic-session','cosmic-settings-daemon','cosmic-applets'} <= lock.keys():
        reasons.append('complete_cohort_lock_required')
    import functools
    import apt_pkg
    apt_pkg.init_system()
    wanted = lock or {name: max((version for n, version in packages if n == name), key=functools.cmp_to_key(apt_pkg.version_compare), default='') for name in
                     ('cosmic-panel', 'cosmic-comp', 'cosmic-session', 'cosmic-settings-daemon', 'cosmic-applets')}
    selected = {}
    for name, version in sorted(wanted.items()):
        if (name, version) not in packages:
            reasons.append('package_unavailable:' + name)
            continue
        row, base = packages[(name, version)]
        src = row.get('Source', name)
        match = re.fullmatch(r'([a-z0-9+.-]+)(?: \(([^()]+)\))?', src)
        source_name, source_version = match.groups() if match else (None, None)
        source_version = source_version or version
        item = dict(version=version, sha256=row['SHA256'], url=child_url(base,row['Filename']),
                    architecture=row['Architecture'], source=dict(name=source_name, version=source_version),
                    depends=row.get('Depends',''), pre_depends=row.get('Pre-Depends',''), provides=row.get('Provides',''))
        # Always verify package bytes, even for a blocked candidate.
        data = checked(get(item['url']), item['sha256'], int(row['Size']))
        item['files'] = package_files(data)
        if config.get('retain_packages'):
            destination = out.parent / 'packages' / (item['sha256'] + '.deb')
            destination.parent.mkdir(exist_ok=True)
            destination.write_bytes(data)
        source_entry = sources.get((source_name,source_version))
        if source_entry:
            source_row, source_base = source_entry
            identities = []
            for line in source_row['Checksums-Sha256'].splitlines():
                if not line.strip(): continue
                checksum, size, filename = line.split()
                url = child_url(source_base, source_row['Directory'] + '/' + filename)
                checked(get(url), checksum, int(size))
                identities.append(dict(name=filename, sha256=checksum, url=url))
            item['source']['archives'] = identities
            item['source']['identity'] = identity(identities)
        else:
            reasons.append('verified_source_unavailable:' + name)
        selected[name] = item
    if lock:
        reasons.extend(dependency_gaps(selected))
    # Let the distribution's solver validate exact installed closure in the
    # retained image too; metadata alone is discovery, never native acceptance.
    cohort = dict(distribution=config['distribution'], suite=config['suite'], architecture=config['architecture'],
                  packages=selected, processes=config.get('processes', {
                      'cosmic-comp':'/usr/bin/cosmic-comp', 'cosmic-session':'/usr/bin/cosmic-session',
                      'cosmic-settings-daemon':'/usr/bin/cosmic-settings-daemon'}))
    from bundle_contract import cohort_id
    candidate_id = cohort_id(cohort)
    result = dict(format='halcyon-cosmic-candidate', version=1, id=candidate_id, cohort=cohort,
                  indexes=releases, state='blocked' if reasons else 'discovered', reasons=sorted(set(reasons)))
    result['event_id'] = identity(dict(candidate=candidate_id, reasons=result['reasons']))
    result['duplicate'] = result['event_id'] in previous.get('events', [])
    result['events'] = sorted(set(previous.get('events', []) + [result['event_id']]))[-1024:]
    out.write_bytes(canonical(result) + b'\n')
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-config', type=Path, required=True)
    parser.add_argument('--previous', type=Path)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    previous = json.loads(args.previous.read_bytes()) if args.previous and args.previous.exists() else {}
    config = json.loads(args.source_config.read_bytes())
    for source in config['archives']:
        source['keyring'] = str((args.source_config.parent / source['keyring']).resolve())
    args.out.parent.mkdir(parents=True, exist_ok=True)
    try:
        result = discover(config, previous, args.out)
    except (OSError, ValueError, KeyError, subprocess.SubprocessError) as error:
        # Deduplicate failures by stable code, never log URLs with credentials,
        # local filenames or arbitrary subprocess exception text.
        reason = str(error) if isinstance(error, ValueError) and re.fullmatch('[a-z_]+', str(error)) else type(error).__name__
        event = identity(dict(config=identity(config), failure=reason))
        result = dict(format='halcyon-cosmic-candidate', version=1, state='blocked', reasons=[reason], event_id=event,
                      duplicate=event in previous.get('events', []), events=sorted(set(previous.get('events', [])+[event]))[-1024:])
        args.out.write_bytes(canonical(result)+b'\n')
    print(json.dumps({k:result[k] for k in ('state','duplicate','event_id','reasons')}))
    return 0 if result['state'] == 'discovered' else 2


if __name__ == '__main__':
    sys.exit(main())

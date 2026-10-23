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
import tempfile
import time
import urllib.parse

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'companion'))
from bundle_contract import canonical, identity
from archive_io import (Limits, Payloads, RetainedFetch, bounded_read, fetch,
                        input_stream, package_files, verify_stream)

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



def child_url(base, path):
    if path.startswith('/') or '..' in Path(path).parts or urllib.parse.urlparse(path).scheme or '?' in path or '#' in path:
        raise ValueError('archive_path')
    return base.rstrip('/') + '/' + path


def verify_release(signed, keyring, fingerprint, now, max_age_days=7, immutable_release_sha256=None):
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
    if immutable_release_sha256 is not None and sha(signed) != immutable_release_sha256:
        raise ValueError('immutable_release_changed')
    # Stable base pockets can be explicitly pinned by reviewed complete signed
    # metadata hash. This never overrides a publisher's Valid-Until deadline.
    immutable = immutable_release_sha256 is not None and not release.get('Valid-Until') and release['Suite'] == release['Codename']
    expires = dt.datetime.max.replace(tzinfo=dt.timezone.utc) if immutable else date + dt.timedelta(days=min(max_age_days, 7))
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
    import io
    output = io.BytesIO()
    verify_stream(get, child_url(base, path), checksum, size, 'index', Limits(), output)
    compressed = output.getvalue()
    if path.endswith(('.gz', '.xz')):
        import io, lzma
        opener = gzip.open if path.endswith('.gz') else lzma.open
        with opener(io.BytesIO(compressed)) as stream:
            data = stream.read(MAX_BYTES + 1)
        if len(data) > MAX_BYTES:
            raise ValueError('expanded_index_limit')
        return paragraphs(data)
    return paragraphs(compressed)



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
    started = time.monotonic()
    real_clock = now is None
    now = now or dt.datetime.now(dt.timezone.utc)
    limits = Limits.configured(config.get('resource_limits', {}))
    payloads = Payloads(get, limits)
    inventories = {}
    releases, packages, sources = [], {}, {}
    missing, reasons = [], []
    for source in config['archives']:
        key = Path(source['keyring'])
        if sha(key.read_bytes()) != source['keyring_sha256']:
            raise ValueError('trust_root_changed')
        base = source['url'].rstrip('/')
        dist = child_url(base, 'dists/' + source['pocket'])
        try:
            with input_stream(get, child_url(dist, 'InRelease')) as stream:
                signed = bounded_read(stream, 4 * 1024 * 1024)
        except FileNotFoundError:
            missing.append(dict(kind='signed-release', url=child_url(dist, 'InRelease')))
            reasons.append('archive_input_missing:' + source['pocket'])
            continue
        release, hashes, expires = verify_release(signed, key, source['fingerprint'], now, immutable_release_sha256=source.get('immutable_release_sha256'))
        if release['Codename'] != source['codename'] or release['Suite'] != source['suite'] or config['architecture'] not in release['Architectures'].split() or release['Origin'] != source['origin']:
            raise ValueError('archive_scope_mismatch')
        provenance = dict(url=dist, sha256=sha(signed), expires=expires, fingerprint=source['fingerprint'],
                          pocket=source['pocket'], suite=release['Suite'], codename=release['Codename'], origin=release['Origin'])
        releases.append(provenance)
        for component in source['components']:
            for kind, stem, destination in (
                    ('binary-index', component + '/binary-' + config['architecture'] + '/Packages', packages),
                    ('source-index', component + '/source/Sources', sources)):
                path = next((stem + suffix for suffix in ('.gz', '.xz', '') if stem + suffix in hashes), None)
                if path is None:
                    missing.append(dict(kind=kind, url=child_url(dist, stem), reason='not_in_signed_release'))
                    continue
                try:
                    rows = download_index(dist, path, hashes, get)
                except FileNotFoundError:
                    missing.append(dict(kind=kind, url=child_url(dist, path), sha256=hashes[path][0], size=hashes[path][1]))
                    continue
                origin = {**provenance, 'index': path, 'index_sha256': hashes[path][0]}
                for row in rows:
                    if kind == 'source-index' or row['Architecture'] in (config['architecture'], 'all'):
                        destination.setdefault((row['Package'], row['Version']), []).append((row, base, origin))
    # A complete image's exact lock is authoritative for dependency resolution.
    # Three latest versions alone never constitute a qualification environment.
    lock = config.get('cohort_lock', {})
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
        entries = packages[(name, version)]
        pin = config.get('package_sha256', {}).get(name)
        if pin:
            entries = [entry for entry in entries if entry[0]['SHA256'] == pin]
        if not entries:
            reasons.append('package_digest_unavailable:' + name)
            continue
        if len({identity({k:entry[0].get(k) for k in ('SHA256','Size','Source','Architecture','Depends','Pre-Depends','Provides')}) for entry in entries}) != 1:
            reasons.append('binary_provenance_collision:' + name)
            continue
        row, base, _ = sorted(entries, key=lambda e: (e[1], e[0]['Filename']))[0]
        src = row.get('Source', name)
        match = re.fullmatch(r'([a-z0-9+.-]+)(?: \(([^()]+)\))?', src)
        source_name, source_version = match.groups() if match else (None, None)
        source_version = source_version or version
        item = dict(version=version, sha256=row['SHA256'], url=child_url(base,row['Filename']),
                    architecture=row['Architecture'], source=dict(name=source_name, version=source_version),
                    provenance=[e[2] for e in entries], depends=row.get('Depends',''), pre_depends=row.get('Pre-Depends',''), provides=row.get('Provides',''))
        # Always verify package bytes, even for a blocked candidate.
        try:
            if config.get('resolve_metadata_only'):
                raise FileNotFoundError()
            with payloads.binary(item['url'], item['sha256'], int(row['Size']), out.parent) as package:
                stats = {}
                item['files'] = package_files(package, limits=limits, stats=stats)
                payloads.account_inventory(item['files'])
                inventories[name] = stats
                if config.get('retain_packages'):
                    destination = out.parent / 'packages' / (item['sha256'] + '.deb')
                    destination.parent.mkdir(exist_ok=True)
                    # Private verified snapshot, same filesystem, atomic publish
                    # only after complete hash AND archive validation.
                    package.replace(destination)
        except FileNotFoundError:
            missing.append(dict(kind='binary', package=name, version=version, url=item['url'], sha256=item['sha256'], size=int(row['Size'])))
        source_entries = sources.get((source_name, source_version), [])
        # Index order never chooses between distinct source bytes.
        source_ids = {identity(sorted(line.strip() for line in entry[0]['Checksums-Sha256'].splitlines() if line.strip())) for entry in source_entries}
        if len(source_ids) > 1:
            reasons.append('source_provenance_collision:' + name)
            item['source']['provenance'] = [e[2] for e in source_entries]
        elif source_entries:
            source_row, source_base, _ = sorted(source_entries, key=lambda e: (e[1], e[0]['Directory']))[0]
            item['source']['provenance'] = [e[2] for e in source_entries]
            identities = []
            for line in source_row['Checksums-Sha256'].splitlines():
                if not line.strip(): continue
                checksum, size, filename = line.split()
                url = child_url(source_base, source_row['Directory'] + '/' + filename)
                try:
                    if config.get('resolve_metadata_only'):
                        raise FileNotFoundError()
                    payloads.source(url, checksum, int(size))
                except FileNotFoundError:
                    missing.append(dict(kind='source-archive', package=source_name, version=source_version, url=url, sha256=checksum, size=int(size)))
                identities.append(dict(name=filename, sha256=checksum, url=url))
            item['source']['archives'] = identities
            item['source']['identity'] = identity([{k:v for k,v in entry.items() if k != 'url'} for entry in identities])
        else:
            reasons.append('verified_source_unavailable:' + name)
        selected[name] = item
    if missing:
        reasons.append('authenticated_inputs_incomplete')
    if config.get('resolve_metadata_only'):
        reasons.append('metadata_resolution_only')
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
                  indexes=releases, missing_inputs=missing, collisions={
                      'binary': {n+'='+v: [dict(record=e[0], provenance=e[2]) for e in rows] for (n,v),rows in packages.items() if (n,v) in wanted.items() and len(rows)>1},
                      'source': {n+'='+v: [dict(record=e[0], provenance=e[2]) for e in rows] for (n,v),rows in sources.items() if len({e[0]['Checksums-Sha256'] for e in rows})>1}}, state='blocked' if reasons else 'discovered', reasons=sorted(set(reasons)))
    result['event_id'] = identity(dict(candidate=candidate_id, reasons=result['reasons']))
    result['duplicate'] = result['event_id'] in previous.get('events', [])
    result['events'] = sorted(set(previous.get('events', []) + [result['event_id']]))[-1024:]
    from catalog_capacity import candidate_capacity, write_candidate
    result['resources'] = dict(limits=vars(limits), payload_bytes_reserved=payloads.bytes,
                               payload_objects=payloads.objects, inventories=inventories,
                               capacity=candidate_capacity(result))
    finished = now + dt.timedelta(seconds=time.monotonic() - started)
    if real_clock:
        finished = max(finished, dt.datetime.now(dt.timezone.utc))
    if any(finished >= dt.datetime.fromisoformat(release['expires']) for release in releases):
        raise ValueError('metadata_expired')
    write_candidate(out, result)
    return result


def retained_fetch(manifest_path):
    """Offline URL->retained bytes; the signed Release still authenticates indexes.

    Missing blobs are reported by discover, never silently fetched from today.
    """
    return RetainedFetch(manifest_path)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-config', type=Path, required=True)
    parser.add_argument('--previous', type=Path)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--retained-inputs', type=Path, help='Offline URL/blob manifest; never fetch missing inputs')
    args = parser.parse_args()
    previous = json.loads(args.previous.read_bytes()) if args.previous and args.previous.exists() else {}
    config = json.loads(args.source_config.read_bytes())
    for source in config['archives']:
        source['keyring'] = str((args.source_config.parent / source['keyring']).resolve())
    args.out.parent.mkdir(parents=True, exist_ok=True)
    try:
        result = discover(config, previous, args.out, get=retained_fetch(args.retained_inputs) if args.retained_inputs else fetch)
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

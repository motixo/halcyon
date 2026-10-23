"""Versioned, local-only bundle integrity and compatibility policy.

No IO in classify_compatibility. Callers supply measured identities, never version
suffixes. A catalog is shipped in the package, not downloaded during a login.
"""
import hashlib
import json
import os
from pathlib import Path
import re
import stat

FORMAT = 'halcyon-islands-bundle'
CATALOG = 'halcyon-islands-catalog'
HELPERS = ('panel_launcher.py', 'bundle_contract.py', 'runtime_identity.py',
           'install_status.py', 'install.py', 'refresh.py', 'selector.py')
FILES = (*HELPERS, 'cosmic-panel-islands', 'catalog.json')
HEX = re.compile(r'[0-9a-f]{64}')


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=True).encode()


def identity(value):
    return hashlib.sha256(canonical(value)).hexdigest()


def regular(path):
    return not any(p.is_symlink() for p in (path, *path.parents)) and path.is_file()


def digest(path):
    if not regular(path) or path.stat().st_size > 512 * 1024 * 1024:
        raise ValueError('not_bounded_regular_file')
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def read_json(path):
    if not os.path.lexists(path):
        raise FileNotFoundError(path)
    if not regular(path) or path.stat().st_size > 16 * 1024 * 1024:
        raise ValueError('not_bounded_record')
    value = json.loads(path.read_bytes())
    if not isinstance(value, dict):
        raise ValueError('not_object')
    return value


def cohort_id(cohort):
    return identity({k: cohort[k] for k in ('distribution', 'suite', 'architecture', 'packages', 'processes')})


def valid_catalog(catalog):
    if catalog.get('format') != CATALOG or catalog.get('version') != 1 or not isinstance(catalog.get('cohorts'), list):
        raise ValueError('catalog_schema')
    seen = set()
    for row in catalog['cohorts']:
        if not isinstance(row, dict):
            raise ValueError('catalog_row')
        if row['id'] != cohort_id(row) or row['id'] in seen:
            raise ValueError('catalog_cohort_identity')
        seen.add(row['id'])
        if row['qualification'] not in ('native-session', 'development', 'source-only'):
            raise ValueError('catalog_qualification')
        if not HEX.fullmatch(row['payload_sha256']):
            raise ValueError('catalog_payload')
        if row['qualification'] == 'native-session':
            # Evidence IDs are allocated before finalization to avoid a digest
            # cycle. Promotion validates the external result for this exact deb.
            evidence = row['evidence']
            if not re.fullmatch(r'[a-zA-Z0-9._-]{1,128}', evidence['id']) or not HEX.fullmatch(evidence['image_sha256']):
                raise ValueError('catalog_evidence')
            if evidence['scope'] != 'complete-session':
                raise ValueError('catalog_scope')
        if not {'cosmic-comp', 'cosmic-session', 'cosmic-settings-daemon'} <= row['processes'].keys():
            raise ValueError('catalog_processes')
        if not {'cosmic-panel', 'cosmic-comp', 'cosmic-session', 'cosmic-settings-daemon'} <= row['packages'].keys():
            raise ValueError('catalog_packages')
        for package in row['packages'].values():
            if not package['version'] or not HEX.fullmatch(package['sha256']) or not package['files']:
                raise ValueError('catalog_package')
            for path, sha in package['files'].items():
                if not path.startswith('/usr/') and not path.startswith('/lib/'):
                    raise ValueError('catalog_file_path')
                if '..' in Path(path).parts or not HEX.fullmatch(sha):
                    raise ValueError('catalog_file_digest')
    return catalog


def verify_bundle(bundle, manifest_name='manifest.json', owner=None):
    manifest = read_json(bundle / manifest_name)
    if owner is not None and (bundle / manifest_name).stat().st_uid != owner:
        raise ValueError('manifest_owner')
    if (bundle / manifest_name).stat().st_mode & 0o022:
        raise ValueError('manifest_writable')
    if manifest.get('format') != FORMAT or manifest.get('version') != 2 or manifest.get('protocol') != 1:
        raise ValueError('bundle_schema')
    if not re.fullmatch(r'[0-9a-f]{40}', manifest.get('upstream', '')):
        raise ValueError('unresolved_upstream')
    if not isinstance(manifest.get('files'), dict):
        raise ValueError('bundle_inventory')
    if set(manifest['files']) != set(FILES) or not re.fullmatch(r'[0-9][A-Za-z0-9.+:~\-]*', manifest['package_version']):
        raise ValueError('bundle_inventory')
    for name, entry in manifest['files'].items():
        if not isinstance(entry, dict) or not isinstance(entry.get('mode'), int) or not isinstance(entry.get('sha256'), str):
            raise ValueError('bundle_entry')
        path = bundle / name
        if digest(path) != entry['sha256'] or stat.S_IMODE(path.stat().st_mode) != entry['mode']:
            raise ValueError('bundle_component_mismatch')
        if owner is not None and path.stat().st_uid != owner:
            raise ValueError('bundle_owner')
        if entry['mode'] & 0o022:
            raise ValueError('bundle_writable')
    payload = {k: v for k, v in manifest['files'].items() if k != 'catalog.json'}
    if identity(payload) != manifest['payload_sha256'] or manifest['binary_sha256'] != payload['cosmic-panel-islands']['sha256']:
        raise ValueError('bundle_payload_mismatch')
    catalog = valid_catalog(read_json(bundle / 'catalog.json'))
    return manifest, catalog


def classify_compatibility(installed, running, manifest, catalog):
    def result(state, reason, cohort=None):
        return dict(version=1, state=state, reason=reason, cohort=cohort)
    try:
        valid_catalog(catalog)
        if manifest.get('format') != FORMAT or manifest.get('version') != 2 or manifest.get('verified') is not True:
            return result('invalid_bundle', 'bundle_not_verified')
        admitted = {r['id']: r for r in catalog['cohorts'] if r['qualification'] == 'native-session'
                    and r['payload_sha256'] == manifest['payload_sha256']}
        current = installed.get('cohort') if installed.get('verified') is True else None
        active = running.get('cohort') if running.get('verified') is True else None
        if current not in admitted:
            return result('unsupported', 'installed_cohort_unqualified')
        if active is None:
            return result('logout_required', 'running_identity_unknown', current)
        if active != current:
            return result('logout_required', 'installed_running_differ', current)
        return result('ready', 'qualified_cohort', current)
    except (KeyError, TypeError, ValueError, AttributeError):
        return result('invalid_bundle', 'catalog_invalid')

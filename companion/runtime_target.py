"""Evidence-bound runtime policy. Full discovery cohorts are never projected away.

This validator performs no installed-image or process IO. A policy is authored
from public authenticated inputs and native dependency review, not inferred by
looking at whatever happens to be installed on a user's desktop.
"""
from pathlib import PurePosixPath
import re

SCOPE = 'islands-runtime'
TARGET_VERSION = 1
MEASUREMENT_VERSION = 4
MAX_FILE_BYTES = 512 * 1024 * 1024
CORE = {'cosmic-comp', 'cosmic-session', 'cosmic-settings-daemon', 'cosmic-notifications'}
KINDS = {'executable', 'library', 'loader', 'resource'}


def public_path(path):
    return (isinstance(path, str) and path.startswith(('/usr/', '/lib/', '/lib64/'))
            and '..' not in path.split('/') and '\0' not in path
            and str(PurePosixPath(path)) == path and len(path) <= 4096)


def target_id(target):
    from bundle_contract import identity
    return identity({k:v for k,v in target.items() if k != 'id'})


def valid_target(target, row):
    from bundle_contract import HEX, identity, runtime_id
    if (not isinstance(target, dict) or target.get('version') != TARGET_VERSION
            or target.get('scope') != SCOPE or target.get('id') != target_id(target)
            or target.get('qualification_cohort_id') != row['id']
            or target.get('payload_sha256') != row['payload_sha256']
            or target.get('evidence_id') != row['evidence']['id']
            or target.get('profile_id') != runtime_id(row['runtime'])
            or target.get('equivalence_id') != identity(row['runtime_compatibility'])):
        raise ValueError('runtime_target_binding')
    if target.get('system') != {k:row[k] for k in ('distribution','suite','architecture')}:
        raise ValueError('runtime_target_system')
    packages = target.get('packages')
    files = target.get('installed_files')
    if not isinstance(packages, dict) or not packages or not isinstance(files, dict) or not files:
        raise ValueError('runtime_target_inventory')
    for name, package in packages.items():
        original = row['packages'].get(name, {})
        if package != {k:original.get(k) for k in ('version','architecture','sha256')}:
            raise ValueError('runtime_target_package_provenance')
        if not package['version'] or not package['architecture'] or not HEX.fullmatch(package['sha256']):
            raise ValueError('runtime_target_package')
    for path, item in files.items():
        if (not public_path(path) or not isinstance(item, dict) or item.get('kind') not in KINDS
                or item.get('resolution') != 'root-contained' or not isinstance(item.get('reason'), str)
                or not 1 <= len(item['reason']) <= 1024 or type(item.get('size')) is not int
                or not 0 <= item['size'] <= MAX_FILE_BYTES or not HEX.fullmatch(item.get('sha256', ''))):
            raise ValueError('runtime_target_file')
        package = item.get('package')
        archive_path = item.get('archive_path')
        original = row['packages'].get(package, {})
        # No "unreadable = equal" or installed-image hash substitution. Public
        # transformed resources need a separately reviewed future schema.
        if (package not in packages or not public_path(archive_path)
                or original.get('files', {}).get(archive_path) != item['sha256']
                or item.get('source_identity') != original.get('source', {}).get('identity')
                or not HEX.fullmatch(item.get('source_identity', ''))):
            raise ValueError('runtime_target_file_provenance')
    profile = row['runtime']
    if profile.get('version') != MEASUREMENT_VERSION or not CORE <= profile['processes'].keys():
        raise ValueError('runtime_target_required_roles')
    if not isinstance(profile.get('host_mapped'), dict):
        raise ValueError('runtime_target_host_mappings')
    for path, sha in profile['host_mapped'].items():
        if not public_path(path) or not HEX.fullmatch(sha):
            raise ValueError('runtime_target_host_mapping')
    code = {f['sha256'] for f in files.values() if f['kind'] in ('executable','library','loader')}
    required = set(profile['host_mapped'].values())
    for process in profile['processes'].values():
        required.add(process['sha256']); required.update(process['mapped'].values())
    if not required <= code:
        raise ValueError('runtime_target_unaccounted_code')
    review = target.get('dependency_review', {})
    if (review.get('method') != 'elf-closure-and-source-review-v1'
            or not HEX.fullmatch(review.get('record_sha256', ''))
            or not isinstance(review.get('roots'), list) or not review['roots']
            or not set(review['roots']) <= files.keys()
            or not isinstance(review.get('dynamic_providers'), list)
            or not set(review['dynamic_providers']) <= files.keys()
            or not isinstance(review.get('resources'), list)
            or not set(review['resources']) <= files.keys()):
        raise ValueError('runtime_target_dependency_review')
    if any(f['kind']=='resource' and path not in review['resources'] for path,f in files.items()):
        raise ValueError('runtime_target_resource_review')
    roots = {files[path]['sha256'] for path in review['roots']}
    if not {p['sha256'] for p in profile['processes'].values()} <= roots:
        raise ValueError('runtime_target_missing_root')
    if target.get('work') != dict(files=len(files), bytes=sum(f['size'] for f in files.values())):
        raise ValueError('runtime_target_work_bound')
    # Scope is explicit; measured constraints are not a whole kernel/firmware
    # assertion. A native driver must validate the exact declared scope.
    scope = target.get('native_scope', {})
    if (not isinstance(scope.get('graphics'), str) or not scope['graphics']
            or not HEX.fullmatch(scope.get('review_sha256', ''))
            or scope.get('constraints_sha256') != identity(profile['constraints'])):
        raise ValueError('runtime_target_native_scope')
    return target


def installed_matches(measurement, row):
    return (measurement.get('format') == 'halcyon-installed-runtime'
            and measurement.get('version') == 1 and measurement.get('scope') == SCOPE
            and measurement.get('verified') is True
            and row.get('runtime_target', {}).get('id') in measurement.get('targets', []))

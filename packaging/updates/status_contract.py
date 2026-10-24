"""Executable handoff model for the future Rust adapter; no runtime integration.

Inputs are measurements supplied by that adapter, not assertions from a remote
server. This module never probes a live desktop, contacts PackageKit or installs.
"""
import subprocess

MAIN = 'cosmic-ext-halcyon'
ISLANDS = MAIN + '-islands'
EXECUTABLE = '/usr/bin/halcyon'
FAILURES = {'offline', 'timeout', 'cancelled', 'authorization-cancelled',
            'missing-packagekit', 'untrusted', 'failed'}


def classify(measurement):
    running = measurement['running']
    package = measurement.get('package') or {}
    same_path = running.get('path') == EXECUTABLE
    known_source = running.get('source_install_verified') is True and not same_path
    if known_source:
        return 'Source', 'source-build'
    if (not same_path or package.get('name') != MAIN or package.get('status') != 'installed'
            or package.get('architecture') != 'amd64' or package.get('owns_executable') is not True):
        return 'Unknown', 'running-installation-unverified'
    digest = running.get('sha256')
    if not digest or digest != package.get('installed_file_sha256') or running.get('deleted'):
        return 'Unknown', 'running-package-changed-restart-to-recheck'
    if measurement.get('enrollment') == 'verified-enabled':
        return 'Apt', 'scoped-source-enabled'
    if measurement.get('enrollment') in ('absent', 'disabled'):
        return 'ManualDeb', 'source-not-enabled'
    return 'Unknown', 'enrollment-unverified'


def reduce_status(measurement, transaction=None):
    channel, reason = classify(measurement)
    result = dict(version=1, channel=channel, reason=reason,
                  running_version=measurement['running'].get('version'),
                  installed_version=(measurement.get('package') or {}).get('version') if channel in ('Apt', 'ManualDeb') else None,
                  candidate_version=None, check='not-checked',
                  pending_islands_login=measurement.get('islands_installed_verified') is True
                      and measurement.get('islands_refresh') == 'logout_required',
                  action={'Apt': 'open-store', 'ManualDeb': 'release-and-enrollment-info',
                          'Source': 'build-info', 'Unknown': 'installation-info'}[channel])
    if channel != 'Apt' or transaction is None:
        return result
    outcome = transaction.get('outcome')
    if outcome in FAILURES:
        result['check'] = outcome
        return result
    # A method reply is NOT transaction success. Require Finished(success), no
    # ErrorCode, and a complete fresh signal stream bound to the request token.
    if outcome != 'finished-success' or transaction.get('error_code') is not None:
        result['check'] = 'checking' if outcome == 'pending' else 'failed'
        return result
    result['check'] = 'up-to-date'
    for package_id in transaction.get('package_ids', []):
        parts = package_id.split(';')
        if len(parts) != 4:
            result['check'] = 'failed'
            result['candidate_version'] = None
            return result
        name, version, arch, _data = parts
        if name != MAIN or arch != 'amd64':
            continue
        compared = subprocess.run(['dpkg', '--compare-versions', version, 'gt', result['installed_version']],
                                  capture_output=True, timeout=5)
        if compared.returncode not in (0, 1):
            result['check'] = 'failed'
            result['candidate_version'] = None
            return result
        if compared.returncode == 0:
            if result['candidate_version'] is not None:
                result['check'] = 'failed'
                result['candidate_version'] = None
                return result
            result['candidate_version'] = version
            result['check'] = 'available'
    return result

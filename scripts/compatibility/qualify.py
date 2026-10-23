#!/usr/bin/env python3
"""Freeze inputs, run an isolated native driver and validate immutable evidence.

The driver is an explicit reviewed executable (never provided by the candidate).
It receives request.json and an output directory. Without a driver, emits the
same runnable request and a blocked result. No build, strip, signing or network.
"""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'companion'))
from bundle_contract import canonical, digest, identity, verify_bundle, cohort_id, measured_configuration
from native_scenarios import SCENARIOS, SYSTEM_UPDATES, validate


def artifact_map(root, artifacts):
    if not isinstance(artifacts, dict):
        raise ValueError('evidence_inventory_missing')
    for name, item in artifacts.items():
        if Path(name).is_absolute() or '..' in Path(name).parts:
            raise ValueError('evidence_path_invalid')
        if digest(root / name) != item['sha256']:
            raise ValueError('evidence_hash_mismatch')
    return artifacts


def main_identity(package):
    from discover import package_files
    fields = subprocess.check_output(['dpkg-deb', '-f', str(package.resolve()), 'Package', 'Version'], text=True, timeout=10)
    control = dict(line.split(': ', 1) for line in fields.splitlines())
    if control['Package'] != 'cosmic-ext-halcyon':
        raise ValueError('wrong_main_package')
    package_hash = digest(package)
    files = package_files(package.read_bytes())
    return dict(package_sha256=package_hash, package_version=control['Version'], files=files,
                executable='/usr/bin/halcyon', executable_sha256=files['/usr/bin/halcyon'])


def input_identities(request):
    return {**request_identities(request), 'candidate_sha256': identity(request['candidate']),
            'image_manifest_sha256': identity(request['image']), 'evidence_id': request['evidence_id'],
            'main_package_sha256': request.get('main_package_sha256'),
            'main_identity_sha256': identity(request.get('main')),
            'runtime_contract_sha256': identity(request['runtime_contract'])}


def validate_result(request, observed, root):
    reasons = []
    if request.get('inputs') != input_identities(request) or observed.get('inputs') != request['inputs']:
        reasons.append('authoritative_inputs_mismatch')
    if observed.get('format') != 'halcyon-native-observation' or observed.get('version') != 1:
        return ['observation_schema']
    if observed.get('request_sha256') != identity(request):
        reasons.append('request_identity_mismatch')
    environment = observed.get('environment', {})
    if environment.get('image_sha256') != request['image']['sha256'] or environment.get('kind') != 'complete-login-session':
        reasons.append('wrong_or_incomplete_image')
    if environment.get('packages') != request['candidate']['cohort']['packages']:
        reasons.append('guest_package_cohort_mismatch')
    if environment.get('graphics') != request['image']['graphics']:
        reasons.append('graphics_scope_mismatch')
    if not environment.get('dpkg_audit_clean') or not environment.get('dependency_closure_verified'):
        reasons.append('guest_dependency_closure_unverified')
    if observed.get('package_sha256') != request['package_sha256'] or observed.get('bundle_sha256') != request['bundle_sha256']:
        reasons.append('guest_artifact_mismatch')
    if not observed.get('offline') or not observed.get('no_host_mounts') or observed.get('secrets_present') is not False:
        reasons.append('isolation_unverified')
    runtime = observed.get('runtime_measurement', {})
    expected_runtime = request.get('runtime_contract')
    if (not expected_runtime or runtime.get('format') != 'halcyon-runtime-measurement' or runtime.get('version') != 3 or runtime.get('phase') != 'running'
            or runtime.get('verified') is not True or identity(expected_runtime['runtime']) not in runtime.get('profiles', [])
            or not measured_configuration(runtime, expected_runtime['runtime'])):
        reasons.append('runtime_measurement_mismatch')
    artifacts = artifact_map(root, observed.get('artifacts'))
    if request.get('main'):
        main = observed.get('main', {})
        live = main.get('running', {})
        if (main.get('installed') != request['main'] or live.get('executable') != request['main']['executable']
                or live.get('sha256') != request['main']['executable_sha256'] or live.get('same_session') is not True
                or not live.get('evidence') or not all(ref in artifacts and artifacts[ref]['kind'] == 'processes' for ref in live.get('evidence', []))):
            reasons.append('main_native_identity_mismatch')
    elif request.get('main_package_sha256'):
        reasons.append('main_identity_missing')
    required = {**SCENARIOS, **(SYSTEM_UPDATES if request.get('main_package_sha256') else {})}
    if request.get('scenarios') != list(required):
        reasons.append('scenario_contract_mismatch')
    reasons.extend(validate(observed.get('scenarios'), artifacts, required))
    return reasons


def run(candidate_path, image_path, package, out, driver=None, main_package=None):
    out.mkdir(parents=True, exist_ok=False)
    candidate = json.loads(candidate_path.read_bytes())
    image = json.loads(image_path.read_bytes())
    if candidate.get('format') != 'halcyon-cosmic-candidate' or candidate.get('version') != 1 or candidate.get('state') != 'discovered' or candidate['id'] != cohort_id(candidate['cohort']):
        raise ValueError('candidate_not_discovered')
    if image.get('format') != 'halcyon-native-image' or image.get('version') != 1 or image.get('kind') != 'complete-login-session':
        raise ValueError('image_not_complete_session')
    for key in ('sha256', 'recipe_sha256', 'retained_inputs_sha256'):
        if not re.fullmatch('[a-f0-9]{64}', image[key]):
            raise ValueError('image_identity_missing')
    if image['cohort_id'] != candidate['id']:
        raise ValueError('image_cohort_mismatch')
    with tempfile.TemporaryDirectory() as work:
        subprocess.run(['dpkg-deb', '-x', str(package.resolve()), work], check=True, timeout=60)
        bundle = Path(work) / 'usr/lib/cosmic-ext-halcyon-islands'
        manifest, catalog = verify_bundle(bundle, 'companion-manifest.json')
        bundle_hash = digest(bundle / 'companion-manifest.json')
    targets = [r for r in catalog['cohorts'] if r['id'] == candidate['id'] and r['payload_sha256'] == manifest['payload_sha256'] and r['qualification'] == 'native-session' and r['evidence']['image_sha256'] == image['sha256']]
    request = dict(format='halcyon-native-request', version=1, candidate=candidate, image=image,
        evidence_id=targets[0]['evidence']['id'] if len(targets) == 1 else None,
        package=str(package.resolve()), package_sha256=digest(package), bundle_sha256=bundle_hash,
        payload_sha256=manifest['payload_sha256'], catalog_sha256=manifest['files']['catalog.json']['sha256'],
        scenarios=list(SCENARIOS), required=dict(network='none', host_mounts=False, secrets=False,
        finalize_before_test=True, preserve_failures=True))
    request['runtime_contract'] = {k: targets[0][k] for k in ('runtime', 'runtime_compatibility')} if len(targets) == 1 else None
    request['main'] = main_identity(main_package) if main_package else None
    request['main_package_sha256'] = digest(main_package) if main_package else None
    request['scenarios'] = list({**SCENARIOS, **(SYSTEM_UPDATES if main_package else {})})
    request['inputs'] = input_identities(request)
    (out / 'request.json').write_bytes(canonical(request)+b'\n')
    reasons = ['native_driver_required']
    observation_hash = None
    if driver:
        driver = driver.resolve()
        # Do not inherit cloud, signing, repository or calendar credentials.
        env = {k:os.environ[k] for k in ('PATH','LANG','LC_ALL','TMPDIR') if k in os.environ}
        env['HOME'] = str(out.resolve() / 'driver-home')
        Path(env['HOME']).mkdir(mode=0o700)
        try:
            result = subprocess.run([str(driver), '--request', str((out/'request.json').resolve()),
                                     '--out', str(out.resolve())], env=env, timeout=90*60)
            reasons = [] if result.returncode == 0 else ['native_driver_failed']
            observed_path = out/'observations.json'
            observed = json.loads(observed_path.read_bytes())
            reasons += validate_result(request, observed, out)
            observation_hash = digest(observed_path)
        except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError):
            reasons = ['native_evidence_invalid_or_incomplete']
    if not request['evidence_id']:
        reasons.append('reviewed_catalog_target_required')
    if (digest(package) != request['package_sha256'] or
            (main_package and digest(main_package) != request['main_package_sha256'])):
        reasons.append('package_changed_during_qualification')
    result = dict(format='halcyon-qualification', version=1,
        state='passed' if not reasons else 'blocked', reasons=reasons, request_sha256=identity(request),
        package_sha256=request['package_sha256'], bundle_sha256=bundle_hash,
        payload_sha256=manifest['payload_sha256'], catalog_sha256=request['catalog_sha256'],
        candidate_id=candidate['id'], image_sha256=image['sha256'], graphics=image['graphics'],
        inputs=request['inputs'], observation_sha256=observation_hash, scope='complete-session', promotion='review-required')
    (out/'result.json').write_bytes(canonical(result)+b'\n')
    return result


def request_identities(request):
    """Authoritative summary derived only from the frozen request."""
    return {**{k: request[k] for k in ('package_sha256', 'bundle_sha256', 'payload_sha256', 'catalog_sha256')},
            'candidate_id': request['candidate']['id'], 'image_sha256': request['image']['sha256'],
            'graphics': request['image']['graphics']}


def promotion(package, evidence, approval, main_package=None):
    result = json.loads((evidence/'result.json').read_bytes())
    request = json.loads((evidence/'request.json').read_bytes())
    observed = json.loads((evidence/'observations.json').read_bytes())
    reviewed = json.loads(approval.read_bytes())
    expected = request_identities(request)
    if (request.get('format') != 'halcyon-native-request' or request.get('version') != 1
            or result.get('format') != 'halcyon-qualification' or result.get('version') != 1
            or request.get('inputs') != input_identities(request)
            or result.get('inputs') != request['inputs']
            or reviewed.get('inputs') != request['inputs']
            or (digest(main_package) if main_package else None) != request.get('main_package_sha256')
            or (main_identity(main_package) if main_package else None) != request.get('main')
            or request['package_sha256'] != digest(package)
            or any(result.get(k) != v for k, v in expected.items())
            or request['candidate']['id'] != cohort_id(request['candidate']['cohort'])
            or request['image']['cohort_id'] != request['candidate']['id']
            or result['state'] != 'passed' or result['scope'] != 'complete-session'
            or result['package_sha256'] != digest(package)
            or result['request_sha256'] != identity(request)
            or result['observation_sha256'] != digest(evidence/'observations.json')
            or validate_result(request, observed, evidence)
            or reviewed.get('decision') != 'approve'
            or reviewed.get('evidence_sha256') != digest(evidence/'result.json')
            or reviewed.get('package_sha256') != digest(package)
            or not reviewed.get('review_reference')):
        raise ValueError('promotion_evidence_not_approved')
    with tempfile.TemporaryDirectory() as work:
        subprocess.run(['dpkg-deb', '-x', str(package.resolve()), work], check=True, timeout=60)
        bundle = Path(work) / 'usr/lib/cosmic-ext-halcyon-islands'
        manifest, catalog = verify_bundle(bundle, 'companion-manifest.json')
        if digest(bundle / 'companion-manifest.json') != request['bundle_sha256'] or manifest['payload_sha256'] != request['payload_sha256'] or manifest['files']['catalog.json']['sha256'] != request['catalog_sha256']:
            raise ValueError('promotion_bundle_mismatch')
        if not any(r['qualification'] == 'native-session' and {k:r[k] for k in ('runtime','runtime_compatibility')} == request['runtime_contract'] and r['payload_sha256'] == request['payload_sha256'] and r['id'] == request['candidate']['id'] and r['qualification'] == 'native-session' and
                   r['evidence']['id'] == request['evidence_id'] and r['evidence']['image_sha256'] == request['image']['sha256']
                   for r in catalog['cohorts']):
            raise ValueError('promotion_catalog_evidence_mismatch')
    if digest(package) != request['package_sha256'] or (main_package and digest(main_package) != request['main_package_sha256']):
        raise ValueError('package_changed_during_promotion')
    return dict(package_sha256=digest(package), evidence_sha256=digest(evidence/'result.json'),
                review_reference=reviewed['review_reference'])


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--candidate',type=Path)
    p.add_argument('--image',type=Path)
    p.add_argument('--package',type=Path,required=True)
    p.add_argument('--out',type=Path)
    p.add_argument('--main-package',type=Path)
    p.add_argument('--driver',type=Path)
    p.add_argument('--evidence',type=Path)
    p.add_argument('--approval',type=Path)
    args=p.parse_args()
    try:
        if args.approval:
            print(json.dumps(promotion(args.package,args.evidence,args.approval,args.main_package))); return 0
        if not all((args.candidate,args.image,args.out)):
            p.error('--candidate, --image, --out required')
        result=run(args.candidate,args.image,args.package,args.out,args.driver,args.main_package)
        print(json.dumps(result)); return 0 if result['state']=='passed' else 2
    except (OSError,ValueError,KeyError,TypeError,subprocess.SubprocessError) as error:
        p.exit(2, 'Qualification blocked: '+type(error).__name__+'\n')


if __name__=='__main__':
    sys.exit(main())

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
from bundle_contract import canonical, digest, identity, verify_bundle, cohort_id
from native_scenarios import SCENARIOS, validate


def artifact_map(root, artifacts):
    if not isinstance(artifacts, dict):
        raise ValueError('evidence_inventory_missing')
    for name, item in artifacts.items():
        if Path(name).is_absolute() or '..' in Path(name).parts:
            raise ValueError('evidence_path_invalid')
        if digest(root / name) != item['sha256']:
            raise ValueError('evidence_hash_mismatch')
    return artifacts


def validate_result(request, observed, root):
    reasons = []
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
    artifacts = artifact_map(root, observed.get('artifacts'))
    reasons.extend(validate(observed.get('scenarios'), artifacts))
    return reasons


def run(candidate_path, image_path, package, out, driver=None):
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
    if digest(package) != request['package_sha256']:
        reasons.append('package_changed_during_qualification')
    result = dict(format='halcyon-qualification', version=1,
        state='passed' if not reasons else 'blocked', reasons=reasons, request_sha256=identity(request),
        package_sha256=request['package_sha256'], bundle_sha256=bundle_hash,
        payload_sha256=manifest['payload_sha256'], catalog_sha256=request['catalog_sha256'],
        candidate_id=candidate['id'], image_sha256=image['sha256'], graphics=image['graphics'],
        observation_sha256=observation_hash, scope='complete-session', promotion='review-required')
    (out/'result.json').write_bytes(canonical(result)+b'\n')
    return result


def promotion(package, evidence, approval):
    result = json.loads((evidence/'result.json').read_bytes())
    request = json.loads((evidence/'request.json').read_bytes())
    observed = json.loads((evidence/'observations.json').read_bytes())
    reviewed = json.loads(approval.read_bytes())
    if (result['state'] != 'passed' or result['scope'] != 'complete-session'
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
        if digest(bundle / 'companion-manifest.json') != request['bundle_sha256'] or manifest['payload_sha256'] != request['payload_sha256']:
            raise ValueError('promotion_bundle_mismatch')
        if not any(r['id'] == request['candidate']['id'] and r['qualification'] == 'native-session' and
                   r['evidence']['id'] == request['evidence_id'] and r['evidence']['image_sha256'] == request['image']['sha256']
                   for r in catalog['cohorts']):
            raise ValueError('promotion_catalog_evidence_mismatch')
    return dict(package_sha256=digest(package), evidence_sha256=digest(evidence/'result.json'),
                review_reference=reviewed['review_reference'])


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--candidate',type=Path)
    p.add_argument('--image',type=Path)
    p.add_argument('--package',type=Path,required=True)
    p.add_argument('--out',type=Path)
    p.add_argument('--driver',type=Path)
    p.add_argument('--evidence',type=Path)
    p.add_argument('--approval',type=Path)
    args=p.parse_args()
    try:
        if args.approval:
            print(json.dumps(promotion(args.package,args.evidence,args.approval))); return 0
        if not all((args.candidate,args.image,args.out)):
            p.error('--candidate, --image, --out required')
        result=run(args.candidate,args.image,args.package,args.out,args.driver)
        print(json.dumps(result)); return 0 if result['state']=='passed' else 2
    except (OSError,ValueError,KeyError,subprocess.SubprocessError) as error:
        p.exit(2, 'Qualification blocked: '+type(error).__name__+'\n')


if __name__=='__main__':
    sys.exit(main())

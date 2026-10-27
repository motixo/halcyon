#!/usr/bin/env python3
"""Verify reviewed immutable artifacts before publication; never rebuild."""
import argparse
import json
from pathlib import Path
import sys
from qualify import promotion
from bundle_contract import digest
from runtime_compatibility import POLICY

# Already published bytes, retained solely for APT metadata renewal. This is
# deliberately independent of distribution.json and candidate-supplied policy.
# New artifacts must pass verify() with the current policy and fresh approval.
PUBLISHED_RELEASES = {
    '6e5facd7eb8cf5cda71d57022ee8f33728148ba102229d66e20c8d8ed4827351': {
        'release': '0.3.0',
        'policy': 'cosmic-1.10-1.11-startup-v2',
        'approval_sha256': '0d9ffca4e176200818a7a7ea5be2b8edce0d017369c50797b4afdbee668e12c3',
    },
    'ea237af6b49635dcb9f0fe03e1523df6aaf66fd332a2729c8a335b8d82cff292': {
        'release': '0.2.5',
        'policy': 'cosmic-1.10-1.11-startup-v1',
        'approval_sha256': '563ac8ecc85fb1d6f29e66e062c7d88393d2cdc67002c05785d42a798eb0e5ab',
    },
}


def verify_published(root, expected):
    """Renew immutable historical publications without admitting new old-policy bytes."""
    published = PUBLISHED_RELEASES.get(expected)
    if published is None:
        return verify(root, expected)
    manifest = root/'release-manifest.json'
    if digest(manifest) != expected:
        raise ValueError('reviewed_manifest_digest_mismatch')
    data = json.loads(manifest.read_bytes())
    if (data.get('format') != 'halcyon-release' or data.get('version') != 1
            or data.get('release') != published['release']
            or data.get('compatibility_policy') != published['policy']):
        raise ValueError('published_release_identity_mismatch')
    artifacts(root, data)  # The pinned manifest binds every package/source/evidence byte.
    if digest(root/'approval.json') != published['approval_sha256']:
        raise ValueError('published_release_approval_mismatch')
    return data


def verify(root, expected):
    manifest=root/'release-manifest.json'
    if digest(manifest)!=expected:raise ValueError('reviewed_manifest_digest_mismatch')
    data=json.loads(manifest.read_bytes())
    if data.get('format')!='halcyon-release' or data.get('version')!=1 or data.get('release')=='0.2.0':
        raise ValueError('release_manifest_invalid')
    package, main = artifacts(root,data)
    if data.get('compatibility_policy') == POLICY['id']:
        from practical_release import verify as verify_practical
        verify_practical(root,data,expected)
        return data
    if data.get('compatibility_policy') is not None: raise ValueError('unknown_release_policy')
    promotion(root/package,root/'native-evidence',root/'approval.json',root/main)
    return data

def artifacts(root,data):
    kinds=set()
    for name,record in data['artifacts'].items():
        if Path(name).name!=name or digest(root/name)!=record['sha256']:raise ValueError('artifact_mismatch')
        kinds.add(record['kind'])
    if not {'main-deb','islands-deb','corresponding-source'} <= kinds:raise ValueError('release_incomplete')
    package=data['islands_package']
    if data['artifacts'][package]['kind']!='islands-deb':raise ValueError('wrong_companion')
    main = [name for name, record in data['artifacts'].items() if record['kind'] == 'main-deb']
    if len(main) != 1:
        raise ValueError('exact_main_package_required')
    return package, main[0]


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--artifacts',type=Path,required=True);p.add_argument('--manifest-sha256',required=True)
    a=p.parse_args()
    try:verify(a.artifacts,a.manifest_sha256)
    except (OSError,ValueError,KeyError,TypeError):sys.exit('Release blocked: immutable artifacts or reviewed native evidence incomplete')

#!/usr/bin/env python3
"""Verify reviewed immutable artifacts before publication; never rebuild."""
import argparse
import json
from pathlib import Path
import sys
from qualify import promotion
from bundle_contract import digest


def verify(root, expected):
    manifest=root/'release-manifest.json'
    if digest(manifest)!=expected:raise ValueError('reviewed_manifest_digest_mismatch')
    data=json.loads(manifest.read_bytes())
    if data.get('format')!='halcyon-release' or data.get('version')!=1 or data.get('release')=='0.2.0':
        raise ValueError('release_manifest_invalid')
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
    promotion(root/package,root/'native-evidence',root/'approval.json',root/main[0])
    return data

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--artifacts',type=Path,required=True);p.add_argument('--manifest-sha256',required=True)
    a=p.parse_args()
    try:verify(a.artifacts,a.manifest_sha256)
    except (OSError,ValueError,KeyError,TypeError):sys.exit('Release blocked: immutable artifacts or reviewed native evidence incomplete')

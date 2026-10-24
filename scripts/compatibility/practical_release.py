"""Final artifact audit for the explicit supported-family product policy.

This is not qualify.promotion and never creates a native-session result. A parent
approval binds FINAL artifact bytes after packaging; historical VM qualification
is reused only as historical evidence, with unchanged native ELF identities.
"""
import json
from pathlib import Path
import subprocess
import tempfile
from bundle_contract import HELPERS, digest, identity, read_json, verify_bundle
from runtime_compatibility import POLICY, validate_bundle

ROOT=Path(__file__).resolve().parents[2]
ACCEPTANCE=ROOT/'companion/compatibility/evidence/accepted-runtime-40071768899.json'
LIMITATIONS=['physical-hardware-not-qualified','cosmic-1.10-not-native-qualified',
             'new-family-startup-helpers-not-native-qualified','not-full-native-matrix']


def audit(root, data, expected):
    """Return a review request; no approval is inferred or written."""
    main=[n for n,v in data['artifacts'].items() if v['kind']=='main-deb']
    islands=data['islands_package']
    if len(main)!=1: raise ValueError('exact_main_package_required')
    accepted=read_json(ACCEPTANCE)
    with tempfile.TemporaryDirectory(prefix='halcyon-release-audit-') as temporary:
        work=Path(temporary)
        for name,destination in ((islands,work/'islands'),(main[0],work/'main')):
            subprocess.run(['dpkg-deb','-x',str((root/name).resolve()),str(destination)],check=True,timeout=60)
        library=work/'islands/usr/lib/cosmic-ext-halcyon-islands'
        manifest,catalog=verify_bundle(library,'companion-manifest.json')
        validate_bundle(manifest,catalog)
        if (manifest['binary_sha256']!=accepted['tested_packages']['islands']['elf_sha256']
                or digest(work/'main/usr/bin/halcyon')!=accepted['tested_packages']['main']['elf_sha256']):
            raise ValueError('native_elf_reuse_mismatch')
        # Scripts and policy are audited against this frozen release source,
        # never mislabeled as integration.5's tested helper payload.
        for name in HELPERS:
            if digest(library/name)!=digest(ROOT/'companion'/name): raise ValueError('release_helper_source_mismatch')
        if digest(library/'catalog.json')!=digest(ROOT/'companion/compatibility/catalog.json'):
            raise ValueError('release_catalog_source_mismatch')
        app=work/'main/usr/lib/cosmic-ext-halcyon'
        for name in ('bundle_contract.py','runtime_identity.py','runtime_target.py','runtime_compatibility.py',
                     'panel_configuration.py','verification.py','refresh.py','install.py','panel_launcher.py'):
            if digest(app/name)!=digest(library/name): raise ValueError('mixed_status_helpers')
        if digest(app/'islands-status.py')!=digest(library/'install_status.py'):
            raise ValueError('mixed_status_helpers')
        return dict(format='halcyon-practical-release-approval',version=1,
            release_manifest_sha256=expected,policy_id=POLICY['id'],policy_sha256=identity(POLICY),
            historical_acceptance_sha256=digest(ACCEPTANCE),
            payload_sha256=manifest['payload_sha256'],catalog_sha256=digest(library/'catalog.json'),
            artifacts={n:v['sha256'] for n,v in data['artifacts'].items()},limitations=LIMITATIONS)


def verify(root,data,expected):
    required=audit(root,data,expected)
    approval=read_json(root/'approval.json')
    if (any(approval.get(k)!=v for k,v in required.items()) or approval.get('decision')!='approve'
            or not isinstance(approval.get('review_reference'),str) or not approval['review_reference'].strip()):
        raise ValueError('practical_release_not_explicitly_approved')


if __name__=='__main__':
    import argparse
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--artifacts',type=Path,required=True)
    args=parser.parse_args();manifest=args.artifacts/'release-manifest.json'
    data=read_json(manifest)
    # Request generation also enforces the real artifact byte bindings.
    from release_gate import artifacts
    artifacts(args.artifacts,data)
    print(json.dumps({**audit(args.artifacts,data,digest(manifest)),
                      'decision':'PENDING','review_reference':''},indent=2))

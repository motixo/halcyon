#!/usr/bin/env python3
"""Trusted local signing/promotion entry point; no hosting or remote publication."""
import argparse
from pathlib import Path
import time
from archive import promote, sign, verify_tree
from build_archive import admit


def revalidate(root, expected, review_sha):
    data = verify_tree(root, expected)
    if data['review']['review_sha256'] != review_sha:
        raise ValueError('independent_review_pin_required')
    candidate = root/'releases'/data['review']['release_manifest_sha256']
    packages, _ = admit(candidate, review_sha)
    # Bound reviewed packages must be the actual pool bytes being promoted.
    from archive import control, sha
    for package in packages:
        record = control(package)
        matches = [r for r in data['packages'] if all(r[k] == record[k] for k in record)]
        if len(matches) != 1 or sha(root/matches[0]['path']) != record['sha256']:
            raise ValueError('reviewed_pool_binding_mismatch')
    if not time.time() - 7*86400 < data['epoch'] <= time.time() + 300:
        raise ValueError('archive_expired_or_future')


if __name__ == '__main__':
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('action', choices=('sign', 'promote'))
    p.add_argument('--archive', type=Path, required=True)
    p.add_argument('--archive-sha256', required=True)
    p.add_argument('--review-sha256', required=True)
    p.add_argument('--public-keyring', type=Path, required=True)
    p.add_argument('--out', type=Path, required=True)
    p.add_argument('--gnupg-home', type=Path)
    p.add_argument('--signing-subkey')
    p.add_argument('--previous-sha256')
    a = p.parse_args()
    revalidate(a.archive, a.archive_sha256, a.review_sha256)
    if a.action == 'sign':
        if not a.gnupg_home or not a.signing_subkey:
            p.error('sign requires --gnupg-home and --signing-subkey')
        sign(a.archive, a.archive_sha256, a.out, a.gnupg_home, a.signing_subkey, a.public_keyring)
    else:
        promote(a.archive, a.archive_sha256, a.public_keyring, a.out, a.previous_sha256)

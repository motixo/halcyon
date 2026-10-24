#!/usr/bin/env python3
"""Read-only local expiry check. Operator owns scheduling and alert routing."""
import argparse
import json
from pathlib import Path
import time
from archive import signing_deadline, verify_signed


def check(root, expected, keyring, warning=48*3600):
    data = verify_signed(root, expected, keyring)
    signed = json.loads((root/'signed.json').read_bytes())
    deadlines = dict(release=data['epoch']+7*86400,
                     signing_key=signing_deadline(keyring, signed['signing_subkey']))
    remaining = int(min(deadlines.values())-time.time())
    return dict(state='expired' if remaining <= 0 else 'renew' if remaining <= warning else 'current',
                remaining_seconds=remaining, deadlines=deadlines)


if __name__ == '__main__':
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--archive', type=Path, required=True)
    p.add_argument('--archive-sha256', required=True)
    p.add_argument('--public-keyring', type=Path, required=True)
    p.add_argument('--warning-seconds', type=int, default=48*3600)
    a = p.parse_args()
    result = check(a.archive, a.archive_sha256, a.public_keyring, a.warning_seconds)
    print(json.dumps(result, sort_keys=True))
    raise SystemExit(0 if result['state'] == 'current' else 2)

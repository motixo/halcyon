#!/usr/bin/env python3
"""Explicit public-fixture download, pinned to immutable 0.2.0 bytes."""
import argparse
import hashlib
from pathlib import Path
import urllib.request

SHA256='1ec258de00228f02d37fa6201bd4ac4050344594cb452ad6d05ada459ea81c5e'
URL='https://github.com/profilaz3/halcyon/releases/download/v0.2.0/cosmic-ext-halcyon-islands_0.2.0_amd64.deb'

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--out',type=Path,required=True);a=p.parse_args()
    if a.out.exists():
        data=a.out.read_bytes()
    else:
        with urllib.request.urlopen(URL,timeout=30) as response:data=response.read(8*1024*1024)
    if hashlib.sha256(data).hexdigest()!=SHA256:raise SystemExit('Released fixture digest mismatch; nothing written')
    a.out.parent.mkdir(parents=True,exist_ok=True)
    if not a.out.exists():
        with a.out.open('xb') as stream:stream.write(data)

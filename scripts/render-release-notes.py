#!/usr/bin/env python3
"""Render reviewable release text locally. Does not build, tag, upload or publish."""
import argparse
from pathlib import Path
import re
from string import Template
from urllib.parse import urljoin


def render(version, changes, calendar, changes_path=None):
    if not re.fullmatch(r'\d+\.\d+\.\d+', version):
        raise ValueError('Use a stable x.y.z version')
    root=Path(__file__).resolve().parents[1]
    if changes_path is not None:
        relative=changes_path.resolve().relative_to(root)
        base=f'https://github.com/profilaz3/halcyon/blob/v{version}/{relative.as_posix()}'
        # Release bodies have no docs/releases base directory. Keep relative
        # documentation links attached to the exact candidate tag.
        changes=re.sub(r'\]\((\.{1,2}/[^\s)]+)\)',
            lambda match: ']('+urljoin(base,match[1])+')', changes)
    if changes.startswith("# "):
        changes=changes.split("\n",1)[-1]
    return Template((root/'.github/RELEASE_TEMPLATE.md').read_text()).substitute(
        VERSION=version,TAG='v'+version,REPOSITORY='profilaz3/halcyon',
        CHANGES=changes.strip(),CALENDAR=calendar.strip())

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--version',required=True)
    parser.add_argument('--changes',type=Path,required=True)
    parser.add_argument('--calendar',required=True,help='Verified configuration and acceptance for this exact candidate')
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    args.output.write_text(render(args.version,args.changes.read_text(),args.calendar,args.changes)+'\n')

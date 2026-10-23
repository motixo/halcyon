#!/usr/bin/python3
"""Map every installed package to source identity and authenticated APT records.
Unchanged ISO-only versions remain explicitly labeled; the retained signed ISO
is their binary payload/provenance, rather than a guessed current .deb URL.
"""
import apt, hashlib, json, pathlib
cache = apt.Cache()
before = {}
for line in pathlib.Path('/var/lib/qualification/before.tsv').read_text().splitlines():
    name, arch, version, *_ = line.split('\t')
    before[(name.split(':')[0], arch)] = version
result = []
for p in cache:
    v = p.installed
    if not v:
        continue
    row = dict(package=p.name, architecture=v.architecture, version=v.version,
               source=v.source_name, source_version=v.source_version,
               unchanged_from_signed_iso=before.get((p.name.split(':')[0], v.architecture)) == v.version,
               origins=[dict(origin=o.origin,label=o.label,archive=o.archive,site=o.site,trusted=o.trusted) for o in v.origins],
               filename=v.filename)
    try:
        row['sha256'] = v.sha256
    except (ValueError, SystemError):
        row['sha256'] = None
    if not row['unchanged_from_signed_iso'] and not any(o.trusted for o in v.origins):
        raise RuntimeError('Changed package lacks authenticated origin: '+p.name)
    result.append(row)
assert any(r['package']=='cosmic-session' and r['version'].startswith('1.11.') for r in result)
json.dump(result, __import__('sys').stdout, indent=2)

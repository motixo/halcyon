#!/usr/bin/env python3
"""Measure discovery inventories and the unchanged 16 MiB catalog contract.

Measurements do not prune packages, grant admission or relax bundle readers.
"""
import argparse
import json
import os
from pathlib import Path
import sys
import tempfile

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'companion'))
from bundle_contract import cohort_id, read_json, valid_catalog

CATALOG_BYTES = 16 * 1024**2  # bundle_contract.read_json; intentionally unchanged
CANDIDATE_BYTES = 128 * 1024**2
ENCODER = json.JSONEncoder(sort_keys=True, separators=(',', ':'), ensure_ascii=True)


def encoded_size(value):
    return sum(len(part.encode('utf-8')) for part in ENCODER.iterencode(value))


def identity_fields(value):
    # Exactly the nonidentity proof-location fields excluded by cohort_id.
    # Measurement only: no alternative candidate/catalog is emitted.
    if isinstance(value, dict):
        return {k: identity_fields(v) for k, v in value.items() if k not in ('url', 'provenance')}
    if isinstance(value, list):
        return [identity_fields(v) for v in value]
    return value


def candidate_capacity(candidate):
    cohort = candidate['cohort']
    packages = cohort['packages']
    identity_cohort = identity_fields(cohort)
    assert cohort_id(identity_cohort) == cohort_id(cohort)
    def envelope(row):
        return dict(format='halcyon-islands-catalog', version=1, cohorts=[row])
    minimum = encoded_size(envelope(identity_cohort))
    return dict(catalog_reader_limit=CATALOG_BYTES,
                packages=len(packages), inventory_complete=bool(packages) and all('files' in p for p in packages.values()),
                inventory_files=sum(len(p.get('files', {})) for p in packages.values()),
                inventory_json_bytes=sum(encoded_size(p.get('files', {})) for p in packages.values()),
                full_cohort_envelope_bytes=encoded_size(envelope(cohort)),
                identity_cohort_envelope_lower_bound_bytes=minimum,
                lower_bound_exceeds_reader=minimum > CATALOG_BYTES,
                final_catalog_required=True)


def catalog_capacity(path, candidate=None):
    size = path.stat().st_size
    result = dict(actual_bytes=size, catalog_reader_limit=CATALOG_BYTES,
                  within_reader=size <= CATALOG_BYTES, schema_valid=False, candidate_bound=False)
    try:
        catalog = valid_catalog(read_json(path))
        result.update(schema_valid=True, cohorts=len(catalog['cohorts']))
        if candidate is not None:
            expected = cohort_id(candidate['cohort'])
            result['candidate_bound'] = candidate.get('id') == expected and any(row['id'] == expected for row in catalog['cohorts'])
    except (ValueError, KeyError, TypeError) as error:
        result['error'] = str(error) if isinstance(error, ValueError) else 'catalog_schema'
    return result


def write_candidate(path, result):
    # Avoid a second whole-record bytes allocation; no partial successful record.
    fd, name = tempfile.mkstemp(prefix='.candidate-', dir=path.parent)
    try:
        with os.fdopen(fd, 'wb') as output:
            total = 1
            for part in ENCODER.iterencode(result):
                data = part.encode('utf-8')
                total += len(data)
                if total > CANDIDATE_BYTES:
                    raise ValueError('candidate_record_limit')
                output.write(data)
            output.write(b'\n')
            output.flush()
            os.fsync(output.fileno())
        os.replace(name, path)
    finally:
        if os.path.exists(name):
            os.unlink(name)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--candidate', type=Path, required=True)
    parser.add_argument('--catalog', type=Path, help='Final exact catalog, including all rows/runtime/evidence')
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    if args.candidate.stat().st_size > CANDIDATE_BYTES:
        raise ValueError('candidate_record_limit')
    candidate = json.loads(args.candidate.read_bytes())
    report = dict(candidate_bytes=args.candidate.stat().st_size, **candidate_capacity(candidate))
    if args.catalog:
        report['catalog'] = catalog_capacity(args.catalog, candidate)
    args.out.write_text(json.dumps(report, indent=2) + '\n')
    # A measurement with no final catalog is explicitly not a fit certificate.
    return 0 if args.catalog and report['inventory_complete'] and report['catalog']['schema_valid'] and report['catalog']['candidate_bound'] else 2


if __name__ == '__main__':
    sys.exit(main())

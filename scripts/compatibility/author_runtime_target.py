#!/usr/bin/python3
"""Offline explicit runtime-target audit; never creates qualification/admission.

Inputs are an authored catalog row, a public extracted sysroot, the exact bundle,
ELF dependency/source review and native scope review. No ldd, executable probe,
package-name heuristic, tracing inference, host installation or network.
"""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys
sys.path.insert(0, str(Path(__file__).resolve().parents[2]/'companion'))
from bundle_contract import read_catalog, read_json, digest, identity, canonical, verify_bundle
from runtime_identity import FileMeasurement
from runtime_target import valid_target


def elf(path):
    # Parse the declared public object, never execute it. readelf's output is
    # captured in a bounded file, with a CPU/wall bound supplied by the caller.
    import tempfile
    import resource
    def limits():
        resource.setrlimit(resource.RLIMIT_FSIZE,(1024*1024,1024*1024))
        resource.setrlimit(resource.RLIMIT_CPU,(2,2))
    with tempfile.TemporaryFile() as output:
        subprocess.run(['/usr/bin/readelf','-W','-l','-d',str(path)], stdout=output,
                       stderr=subprocess.DEVNULL, env={'LC_ALL':'C','PATH':'/usr/bin:/bin'},
                       timeout=3, check=True, preexec_fn=limits)
        output.seek(0); text=output.read(1024*1024+1).decode()
    if len(text)>1024*1024: raise ValueError('elf_metadata_limit')
    interpreter=re.findall(r'Requesting program interpreter: ([^\]]+)',text)
    needed=re.findall(r'\(NEEDED\).*\[([^\]]+)\]',text)
    return dict(interpreter=interpreter[0] if interpreter else None, needed=sorted(set(needed)),
                rpath=re.findall(r'\(RPATH\).*\[([^\]]*)\]',text),
                runpath=re.findall(r'\(RUNPATH\).*\[([^\]]*)\]',text))


def audit(catalog, cohort, root, bundle, dependency_path, native_path):
    row=next(r for r in catalog['cohorts'] if r['id']==cohort)
    target=valid_target(row['runtime_target'],row)
    manifest,bundled=verify_bundle(bundle,'companion-manifest.json')
    if catalog!=bundled or manifest['payload_sha256']!=target['payload_sha256']:
        raise ValueError('authoring_bundle_mismatch')
    dependency=read_json(dependency_path); native=read_json(native_path)
    if (digest(dependency_path)!=target['dependency_review']['record_sha256']
            or digest(native_path)!=target['native_scope']['review_sha256']):
        raise ValueError('review_record_mismatch')
    if (dependency.get('format')!='halcyon-runtime-dependency-review' or dependency.get('version')!=1
            or native.get('format')!='halcyon-runtime-scope-review' or native.get('version')!=1
            or native.get('graphics')!=target['native_scope']['graphics']
            or native.get('constraints')!=row['runtime']['constraints']
            or not native.get('runtime_discriminators') or not native.get('source_review')):
        raise ValueError('explicit_scope_review_required')
    files=target['installed_files']; code={p for p,f in files.items() if f['kind']!='resource'}
    # The exact payload is a code root too, but never a pre-existing process.
    objects=code|{'@payload'}
    if set(dependency.get('objects',{}))!=objects:
        raise ValueError('incomplete_elf_object_inventory')
    review=target['dependency_review']
    if (dependency.get('roots')!=review['roots'] or dependency.get('dynamic_providers')!=review['dynamic_providers']
            or dependency.get('resources')!=review['resources'] or not dependency.get('source_review')):
        raise ValueError('dependency_source_review_required')
    edges={}; total=0
    with FileMeasurement(root=root,max_files=len(files),max_bytes=target['work']['bytes']) as measured:
        for path,item in files.items():
            actual=measured.resolve(root/path.lstrip('/'))
            if measured.hash(actual,expected_size=item['size'])!=item['sha256']:
                raise ValueError('offline_public_file_mismatch')
            if path in code: edges[path]=elf(actual)
        edges['@payload']=elf(bundle/'cosmic-panel-islands')
        for path,observed in edges.items():
            declaration=dependency['objects'][path]
            if {k:declaration.get(k) for k in observed}!=observed:
                raise ValueError('elf_declaration_mismatch')
            resolved=declaration.get('resolved',{})
            required=set(observed['needed'])|({observed['interpreter']} if observed['interpreter'] else set())
            if set(resolved)!=required or not set(resolved.values())<=code:
                raise ValueError('elf_dependency_unaccounted')
            # Resolution choices require source/native review; a DT_NEEDED name
            # alone cannot prove ld.so cache, IFUNC or dlopen provider behavior.
        measured.check(); total=measured.bytes
    return dict(format='halcyon-runtime-authoring-audit',version=1,state='STRUCTURAL-NOT-QUALIFIED',
        qualification_cohort_id=row['id'],runtime_target_id=target['id'],payload_sha256=manifest['payload_sha256'],
        catalog_sha256=manifest['files']['catalog.json']['sha256'],
        dependency_review_sha256=digest(dependency_path),native_review_sha256=digest(native_path),
        files=len(files),declared_bytes=target['work']['bytes'],read_bytes=total,
        largest_file=max(f['size'] for f in files.values()),elf_objects=len(objects),
        note='Offline public-byte/ELF graph check only. Native resolution, scope, lifecycle and positive performance remain required.')


def main():
    p=argparse.ArgumentParser(description=__doc__)
    for name in ('catalog','root','bundle','dependency-review','native-review','out'):p.add_argument('--'+name,type=Path,required=True)
    p.add_argument('--cohort',required=True)
    args=p.parse_args()
    result=audit(read_catalog(args.catalog),args.cohort,args.root.resolve(),args.bundle,args.dependency_review,args.native_review)
    with args.out.open('xb') as out:out.write(canonical(result)+b'\n')
    print(json.dumps(result))
if __name__=='__main__':main()

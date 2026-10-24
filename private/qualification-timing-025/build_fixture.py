"""Guest-only dev data package. No native ELF, scripts, services or admission."""
import copy,gzip,hashlib,json,pathlib,shutil,subprocess,sys
H=pathlib.Path(__file__).resolve().parent;sys.path.insert(0,str(H/'frozen'))
import bundle_contract as b
PINS=json.loads((H/'input-pins.json').read_text())
def sha(p):
 with p.open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def build(dest,rows):
 for name,want in PINS['frozen_helpers'].items():assert sha(H/'frozen'/name)==want
 data=gzip.decompress((H/'cohort.json.gz').read_bytes());assert hashlib.sha256(data).hexdigest()==PINS['derived_envelope_sha256']
 c=json.loads(data);assert c['id']==PINS['cohort'] and b.cohort_id(c['cohort'])==c['id']
 root=dest/('pkg'+str(rows));lib=root/'usr/lib/cosmic-ext-halcyon-islands';lib.mkdir(parents=True)
 version='0.2.5~dev.native-timing.r1.'+str(rows)
 for name in b.HELPERS:
  shutil.copyfile(H/'frozen'/name,lib/name);(lib/name).chmod(0o755 if name in ('install.py','panel_launcher.py','selector.py') else 0o644)
 (lib/'cosmic-panel-islands').write_bytes(b'#!/bin/sh\n# UNQUALIFIED timing stub. Never launched.\nexit 125\n');(lib/'cosmic-panel-islands').chmod(0o755)
 files={p.name:dict(sha256=b.digest(p),mode=p.stat().st_mode&0o777) for p in lib.iterdir()};payload=b.identity(files)
 row=copy.deepcopy(c['cohort']);row.update(id=c['id'],qualification='development',payload_sha256=payload,evidence=dict(id='TIMING-DEVELOPMENT-NOT-QUALIFIED',image_sha256='4b26c9e9f3430a070d0c48d781f127837b3ce300c62384179aa5a9eeb51a1cc6',scope='complete-session'))
 profile=dict(version=3,processes={name:dict(executable=path,sha256=row['packages'][name]['files'][path],mapped={path:row['packages'][name]['files'][path]},count=1,lifecycle='session') for name,path in row['processes'].items()},configuration=dict(resolver=b.CONFIGURATION_RESOLVER,native_source=b.CONFIGURATION_SOURCE,max_outputs=1,applets={}),constraints={},unmeasured=['previously-read-resources','kernel-bytes','driver-firmware','anonymous-code'])
 row['runtime']=profile;row['runtime_compatibility']=[dict(id=b.runtime_id(profile),evidence_id=row['evidence']['id'],reason='Structural profile only; deliberately unqualified; no native ABI evidence')]
 cat=dict(format=b.CATALOG,version=1,cohorts=[row])
 if rows==2:
  second=copy.deepcopy(row);second['distribution']='structural-NOT-QUALIFIED';second['id']=b.cohort_id(second);cat['cohorts'].append(second)
 b.valid_catalog(cat);assert cat['cohorts'][0]['packages']==c['cohort']['packages']
 (lib/'catalog.json').write_bytes(b.canonical(cat)+b'\n');(lib/'catalog.json').chmod(0o644);files['catalog.json']=dict(sha256=sha(lib/'catalog.json'),mode=0o644)
 manifest=dict(format=b.FORMAT,version=2,protocol=1,upstream=b.CONFIGURATION_SOURCE['upstream'],runtime_probe=1,package_version=version,binary_sha256=files['cosmic-panel-islands']['sha256'],payload_sha256=payload,files=files)
 for name in ('companion-manifest.json','manifest.json'):(lib/name).write_bytes(b.canonical(manifest)+b'\n');(lib/name).chmod(0o644)
 b.verify_bundle(lib,'companion-manifest.json')
 (root/'usr/bin').mkdir(parents=True);(root/'usr/bin/halcyon-islands').write_text('#!/bin/sh\n# Timing fixture sentinel; no enable operation.\nexit 125\n');(root/'usr/bin/halcyon-islands').chmod(0o755)
 (root/'DEBIAN').mkdir();(root/'DEBIAN/control').write_text(f'Package: cosmic-ext-halcyon-islands\nVersion: {version}\nArchitecture: amd64\nMaintainer: Halcyon infrastructure fixture\nDescription: UNQUALIFIED disposable timing data; never launch\n')
 out=dest/('fixture-'+str(rows)+'.deb');subprocess.run(['dpkg-deb','--root-owner-group','-Zgzip','--build',str(root),str(out)],check=True,stdout=subprocess.DEVNULL)
 return dict(rows=rows,package_version=version,deb_sha256=sha(out),deb_bytes=out.stat().st_size,catalog_sha256=files['catalog.json']['sha256'],catalog_bytes=(lib/'catalog.json').stat().st_size,manifest_sha256=sha(lib/'manifest.json'),payload_sha256=payload,qualified=False,native_executable=False,maintainer_scripts=False,autostart=False,selection=False)
if __name__=='__main__':
 dest=pathlib.Path(sys.argv[1]);dest.mkdir(exist_ok=True);print(json.dumps([build(dest,n) for n in (1,2)],indent=2))

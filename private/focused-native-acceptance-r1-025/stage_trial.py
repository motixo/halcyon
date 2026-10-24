"""Parent-authorized isolated test designation; no qualification result or package mutation."""
import pathlib,json,gzip,hashlib,shutil,sys,os
H=pathlib.Path('/var/lib/ps025'); B=pathlib.Path('/usr/lib/cosmic-ext-halcyon-islands')
def sha(p):
 with p.open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def write(p,obj):
 with p.open('x') as f:json.dump(obj,f,sort_keys=True,separators=(',',':'));f.write('\n')
 p.chmod(0o644)
def rebind(catalog,receipt,pins,run_id):
 from bundle_contract import identity,valid_catalog,runtime_id
 from runtime_target import target_id
 assert run_id.isdecimal() or run_id=='LOCAL-FIXTURE'
 row=catalog['cohorts'][0];assert len(catalog['cohorts'])==1 and row['qualification']=='development'
 eid='pending-native-test-'+run_id
 original={'target_id':row['runtime_target']['id'],'evidence':row['evidence'],'payload':row['payload_sha256']}
 row['payload_sha256']=receipt['payload_sha256'];row['qualification']='native-session'
 row['evidence']={'id':eid,'scope':'complete-session','image_sha256':pins['image_sha256'],'qualified':False,'state':'PENDING-TEST-AUTHORIZATION-NOT-RESULT'}
 row['runtime_compatibility']=[{'id':runtime_id(row['runtime']),'evidence_id':eid,'reason':'Parent-authorized isolated production-check trial; no prior pass or promotion.'}]
 t=row['runtime_target'];t.update(payload_sha256=row['payload_sha256'],evidence_id=eid,equivalence_id=identity(row['runtime_compatibility']));t['id']=target_id(t)
 valid_catalog(catalog)
 return {'format':'halcyon-isolated-native-test-authorization','version':1,'state':'PENDING-OBSERVATION','qualified':False,'run_id':run_id,'image_sha256':pins['image_sha256'],'source':receipt['packaging_source_commit'],'payload_sha256':receipt['payload_sha256'],'shipping_manifest_sha256':receipt['manifest_sha256'],'shipping_catalog_sha256':receipt['catalog_sha256'],'target_id':t['id'],'evidence_id':eid,'prior_structural':original,'scope':'focused ordinary acceptance; no full complete-session qualification result','schema_scope_note':'complete-session/native-session are existing test predesignation fields, not assertions of passed observations'}
def main():
 assert os.getuid()==0
 run_id=(H/'run-id.txt').read_text().strip();assert run_id.isdecimal()
 receipt=json.loads((H/'integration2.json').read_text());pins=json.loads((H/'structural-pins.json').read_text())
 for n,h in pins['records'].items():assert sha(H/n)==h
 for n,h in receipt['helper_hashes'].items():assert sha(B/n)==h
 assert sha(B/'companion-manifest.json')==receipt['manifest_sha256'] and sha(B/'catalog.json')==receipt['catalog_sha256']
 sys.path.insert(0,str(B));from bundle_contract import verify_bundle,digest
 original,shipping=verify_bundle(B,'companion-manifest.json');assert not shipping['cohorts']
 with gzip.open(H/'structural-catalog.json.gz','rb') as f:raw=f.read(32*1024**2+1)
 assert len(raw)<=32*1024**2 and hashlib.sha256(raw).hexdigest()==pins['catalog_sha256']
 catalog=json.loads(raw);authorization=rebind(catalog,receipt,pins,run_id)
 out=H/'test-stage';out.mkdir(mode=0o755)
 for n in original['files']:
  if n!='catalog.json':shutil.copy2(B/n,out/n)
 write(out/'catalog.json',catalog)
 manifest=json.loads((B/'companion-manifest.json').read_text());manifest['files']['catalog.json']['sha256']=sha(out/'catalog.json')
 write(out/'manifest.json',manifest);write(out/'companion-manifest.json',manifest)
 verified,_=verify_bundle(out);assert verified['payload_sha256']==receipt['payload_sha256']
 authorization.update(test_manifest_sha256=sha(out/'manifest.json'),test_catalog_sha256=sha(out/'catalog.json'),record_pins=pins)
 write(H/'results/test-stage-authorization.json',authorization)
 write(H/'results/test-stage-manifest.json',manifest)
 write(H/'results/test-stage-target.json',catalog['cohorts'][0]['runtime_target'])
 assert sha(B/'companion-manifest.json')==receipt['manifest_sha256'] and sha(B/'catalog.json')==receipt['catalog_sha256']
 print(json.dumps(authorization))
if __name__=='__main__':main()

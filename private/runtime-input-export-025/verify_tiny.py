"""Supported upload-action result: actual bounded API ZIP retrieval before restore."""
import os,json,pathlib,urllib.request,urllib.error,hashlib,zipfile
W=pathlib.Path(os.environ['WORK']);aid=int(os.environ['TINY_ID']);dig=os.environ['TINY_DIGEST'];token=os.environ['GH_TOKEN'];url=f'https://api.github.com/repos/profilaz3/halcyon/actions/artifacts/{aid}';headers={'Authorization':'Bearer '+token,'Accept':'application/vnd.github+json'}
class NoRedirect(urllib.request.HTTPRedirectHandler):
 def redirect_request(self,*a,**kw):return None
def main():
 with urllib.request.urlopen(urllib.request.Request(url,headers=headers),timeout=15) as r:m=json.load(r)
 assert m['workflow_run']['id']==int(os.environ['GITHUB_RUN_ID']) and m['digest']=='sha256:'+dig and not m['expired'] and 0<m['size_in_bytes']<1048576
 try:
  r=urllib.request.build_opener(NoRedirect).open(urllib.request.Request(url+'/zip',headers=headers),timeout=15);r.close();raise RuntimeError('redirect missing')
 except urllib.error.HTTPError as e:
  if e.code!=302:raise RuntimeError('artifact API retrieval failed') from None
  location=e.headers['Location'];e.close()
 assert location.startswith('https://');p=W/'tiny.zip';h=hashlib.sha256();n=0
 with urllib.request.urlopen(location,timeout=20) as r,p.open('xb') as f:
  while b:=r.read(65536):
   n+=len(b);assert n<=m['size_in_bytes'];f.write(b);h.update(b)
 assert n==m['size_in_bytes'] and h.hexdigest()==dig
 with p.open('rb') as f:assert hashlib.file_digest(f,'sha256').hexdigest()==dig
 with zipfile.ZipFile(p) as z:assert z.namelist()==['tiny-receipt.json'] and z.read('tiny-receipt.json')==(W/'evidence/tiny-receipt.json').read_bytes()
 (W/'evidence/tiny-roundtrip.json').write_text(json.dumps({'passed':True,'artifact':aid,'sha256':dig,'bytes':n,'api_retrieved':True,'independent_rehash':True,'exact_member_match':True,'credentials_recorded':False},indent=2)+'\n');print('Tiny actual upload/API retrieval/hash/member round trip passed')
try:main()
except BaseException as e:
 if isinstance(e,SystemExit):raise
 print('Tiny preflight failed: '+type(e).__name__);raise SystemExit(1) from None

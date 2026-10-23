#!/usr/bin/env python3
"""Host-only authenticated bounded range restore. Never logs credentials/redirects."""
import concurrent.futures,json,os,pathlib,shutil,threading,time,urllib.request,urllib.error,subprocess,hashlib
from common import ARTIFACT,SIZE,ZIP_SHA,HERE,extract_verified
W=pathlib.Path(os.environ['WORK']);E=W/'evidence'
TOKEN=os.environ['GH_TOKEN'];API=f'https://api.github.com/repos/profilaz3/halcyon/actions/artifacts/{ARTIFACT}'
lock=threading.Lock();url=None;url_at=0;etag=None;received=0;deadline=time.monotonic()+1800
class NoRedirect(urllib.request.HTTPRedirectHandler):
 def redirect_request(self,*a,**kw):return None
opener=urllib.request.build_opener(NoRedirect)
def api():
 try:
  with urllib.request.urlopen(urllib.request.Request(API,headers={'Authorization':'Bearer '+TOKEN}),timeout=30) as r:m=json.load(r)
 except Exception:raise RuntimeError('artifact metadata request failed') from None
 if m['id']!=ARTIFACT or m['size_in_bytes']!=SIZE or m.get('digest')!='sha256:'+ZIP_SHA or m['expired'] or m['workflow_run']['id']!=39963889546:raise RuntimeError('artifact identity mismatch')
 return m

def location(force=False):
 global url,url_at
 with lock:
  if url and not force and time.monotonic()-url_at<240:return url
  api()
  try:
   r=opener.open(urllib.request.Request(API+'/zip',headers={'Authorization':'Bearer '+TOKEN}),timeout=30);r.close();raise RuntimeError('missing redirect')
  except urllib.error.HTTPError as e:
   if e.code!=302:raise RuntimeError('artifact redirect status '+str(e.code)) from None
   u=e.headers['Location'];e.close()
  if not u.startswith('https://'):raise RuntimeError('insecure redirect')
  url=u;url_at=time.monotonic();return url

def chunk(start,n):
 global etag,received
 for attempt in range(3):
  if time.monotonic()>deadline:raise RuntimeError('restore time bound')
  try:
   headers={'Range':f'bytes={start}-{start+n-1}','Accept-Encoding':'identity'}
   if etag:headers['If-Match']=etag
   with opener.open(urllib.request.Request(location(attempt>0),headers=headers),timeout=45) as r:
    if r.status!=206 or r.headers.get('Content-Range')!=f'bytes {start}-{start+n-1}/{SIZE}' or int(r.headers.get('Content-Length',-1))!=n or r.headers.get('Content-Encoding','identity')!='identity':raise RuntimeError('unsafe range response')
    tag=r.headers.get('ETag')
    with lock:
     if not tag or tag.startswith('W/'):raise RuntimeError('strong etag required')
     if etag is None:etag=tag
     elif etag!=tag:raise RuntimeError('etag changed')
    data=bytearray()
    while len(data)<n:
     b=r.read(min(1024*1024,n-len(data)))
     if not b:raise OSError('short range')
     with lock:
      received+=len(b)
      if received>SIZE+1024**3:raise RuntimeError('retry byte bound')
     data.extend(b)
     if time.monotonic()>deadline:raise RuntimeError('restore deadline')
   return data
  except RuntimeError:raise
  except Exception:
   if attempt==2:raise RuntimeError('range failed after three attempts') from None
   time.sleep(2)

def main():
 if shutil.disk_usage(W).free<70_000_000_000:raise RuntimeError('need 70GB before download')
 m=api();(E/'original-artifact-api.json').write_text(json.dumps(m,indent=2)+'\n')
 path=W/'original.zip.partial';size=16*1024*1024;offset=0;next_offset=0;h=hashlib.sha256();last=0
 with path.open('xb') as f,concurrent.futures.ThreadPoolExecutor(max_workers=16) as ex:
  pending={}
  for _ in range(16):
   n=min(size,SIZE-next_offset);pending[next_offset]=ex.submit(chunk,next_offset,n);next_offset+=n
  while offset<SIZE:
   b=pending.pop(offset).result()
   if shutil.disk_usage(W).free<35_000_000_000:raise RuntimeError('restore disk floor')
   f.write(b);h.update(b);offset+=len(b)
   if next_offset<SIZE:
    n=min(size,SIZE-next_offset);pending[next_offset]=ex.submit(chunk,next_offset,n);next_offset+=n
   if time.monotonic()-last>30 or offset==SIZE:print(json.dumps({'phase':'download','bytes':offset,'total':SIZE}),flush=True);last=time.monotonic()
  f.flush();os.fsync(f.fileno())
 if offset!=SIZE or h.hexdigest()!=ZIP_SHA:raise RuntimeError('transfer digest mismatch')
 archive=W/'original.zip';os.rename(path,archive);archive.chmod(0o444)
 files=extract_verified(archive,W/'original')
 (E/'original-input-verification.json').write_text(json.dumps({'artifact':ARTIFACT,'size':SIZE,'zip_sha256':ZIP_SHA,'network_bytes':received,'independent_zip_rehash':True,'top_level_files':files},indent=2)+'\n')
 r=subprocess.run(['gpgv','--homedir',str(W),'--keyring',str(W/'original/pop.gpg'),str(W/'original/SHA256SUMS.gpg'),str(W/'original/SHA256SUMS')],capture_output=True)
 (E/'iso-signature.txt').write_bytes(r.stdout+r.stderr)
 if r.returncode:raise RuntimeError('ISO checksum signature invalid')
 print('Original ZIP, all retained files and ISO checksum signature verified',flush=True)
if __name__=='__main__':
 try:main()
 except Exception as e:
  # urllib exceptions may contain ephemeral signed URLs; never print them.
  reason=str(e) if isinstance(e,(RuntimeError,ValueError)) else type(e).__name__
  if 'https://' in reason:reason=type(e).__name__
  (E/'restore-failure.json').write_text(json.dumps({'error':reason,'phase':'restore','qualified':False})+'\n');print('Restore failed: '+reason,flush=True);raise SystemExit(1)

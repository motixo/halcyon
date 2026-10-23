#!/usr/bin/python3
"""Bounded ranged artifact transport. Credentials/redirect URLs never reach logs.
Verify the immutable Actions ZIP digest before extraction, then restore.sh checks
each retained file against the independent evidence artifact's manifest.
"""
import hashlib,json,os,pathlib,stat,subprocess,sys,time,urllib.error,urllib.request,zipfile

def unpack_verified(archive, destination, expected):
    with open(archive,'rb') as stream:
        actual=hashlib.file_digest(stream,'sha256').hexdigest()
    if expected != 'sha256:'+actual:
        raise RuntimeError('Actions ZIP digest mismatch')
    with zipfile.ZipFile(archive) as z:
        for item in z.infolist():
            p=pathlib.PurePosixPath(item.filename)
            if p.is_absolute() or '..' in p.parts or stat.S_ISLNK(item.external_attr >> 16):
                raise RuntimeError('Unsafe artifact member')
        z.extractall(destination)

def main():
    repo,run,name,destination=sys.argv[1:]
    if not run.isdigit():raise ValueError('Numeric run ID required')
    token=os.environ['GH_TOKEN']
    headers={'Authorization':'Bearer '+token,'Accept':'application/vnd.github+json','X-GitHub-Api-Version':'2022-11-28'}
    def api(path):
        with urllib.request.urlopen(urllib.request.Request('https://api.github.com/repos/'+repo+path,headers=headers),timeout=30) as r:
            return json.load(r)
    artifacts=api('/actions/runs/'+run+'/artifacts?per_page=100')['artifacts']
    item=next(a for a in artifacts if a['name']==name and not a['expired'])
    if not item.get('digest','').startswith('sha256:'):raise RuntimeError('Missing Actions ZIP digest')
    dest=pathlib.Path(destination);dest.mkdir(parents=True,exist_ok=True)
    archive=dest.parent/(name+'.zip')
    class NoRedirect(urllib.request.HTTPRedirectHandler):
        def redirect_request(self,*args,**kwargs):return None
    opener=urllib.request.build_opener(NoRedirect)
    for attempt in range(2):
        try:
            opener.open(urllib.request.Request(item['archive_download_url'],headers=headers),timeout=30)
            raise RuntimeError('Expected artifact redirect')
        except urllib.error.HTTPError as e:
            if e.code != 302:raise RuntimeError('Artifact redirect API status '+str(e.code)) from None
            location=e.headers['Location']
        if not location.startswith('https://'):raise RuntimeError('Non-HTTPS artifact location')
        print(json.dumps({'artifact':item['id'],'bytes':item['size_in_bytes'],'attempt':attempt+1,'phase':'parallel-download'}),flush=True)
        started=time.monotonic()
        # Capture all subprocess text: aria2 errors can contain ephemeral SAS
        # URLs. Only our non-sensitive progress/result metadata is emitted.
        try:
            p=subprocess.run(['aria2c','--quiet=true','--max-connection-per-server=16','--split=16',
                '--min-split-size=16M','--file-allocation=none','--continue=true','--auto-file-renaming=false',
                '--max-tries=2','--timeout=30','--dir='+str(archive.parent),'--out='+archive.name,location],
                stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=600)
        except subprocess.TimeoutExpired:
            raise RuntimeError('Parallel artifact transfer exceeded ten-minute bound') from None
        print(json.dumps({'artifact':item['id'],'phase':'download-result','returncode':p.returncode,'seconds':round(time.monotonic()-started)}),flush=True)
        if p.returncode==0:break
    else:raise RuntimeError('Parallel artifact download failed after bounded retry')
    unpack_verified(archive,dest,item['digest'])
    archive.unlink()
    print(json.dumps({'artifact':item['id'],'phase':'verified-and-extracted','digest':item['digest']}),flush=True)

if __name__=='__main__':main()

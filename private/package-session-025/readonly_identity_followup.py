"""Prepared follow-up for an authorized isolated guest, not executed in r2.
Run as the ordinary session user. Reads metadata only for chrony.keys, never keys.
No installation, configuration change, networking, admission or mutation.
"""
import hashlib,json,os,pathlib,stat,subprocess
if os.geteuid()==0:raise SystemExit('Run as the ordinary guest session user; do not elevate this probe')
def command(args):
 p=subprocess.run(args,capture_output=True,text=True,timeout=5)
 return {'returncode':p.returncode,'stdout':p.stdout.strip(),'stderr':p.stderr.strip()}
rows=[]
for path in ('/usr/lib/os-release','/usr/share/chrony/chrony.keys'):
 p=pathlib.Path(path);row={'path':path,'uid':os.getuid(),'readable_by_this_user':os.access(p,os.R_OK)}
 try:
  ls=p.lstat();st=p.stat();row.update(lstat_mode=oct(stat.S_IMODE(ls.st_mode)),uid_owner=st.st_uid,gid_owner=st.st_gid,target_mode=oct(stat.S_IMODE(st.st_mode)),target_bytes=st.st_size,resolved=str(p.resolve()),link=os.readlink(p) if p.is_symlink() else None)
 except OSError as e:row['metadata_error']={'type':type(e).__name__,'errno':e.errno}
 row['package_owner']=command(['/usr/bin/dpkg-query','-S',path])
 row['diversion']=command(['/usr/bin/dpkg-divert','--list',path])
 if path=='/usr/lib/os-release':
  with p.open('rb') as f:row['sha256']=hashlib.file_digest(f,'sha256').hexdigest()
 else:row['content_read']=False
 rows.append(row)
print(json.dumps({'purpose':'explain retained identity refusals; NOT admission','rows':rows},indent=2))

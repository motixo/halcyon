#!/usr/bin/env python3
"""Runs only inside the NEW offline disposable guest after lock file transfer."""
import hashlib,json,os,pathlib,subprocess,sys
from common import purge_changes

def output(args):return subprocess.check_output(args,text=True)
def installed():
 raw=output(['dpkg-query','-W','-f=${Package}\t${Version}\t${db:Status-Status}\n'])
 return {a:b for a,b,c in (line.split('\t') for line in raw.splitlines()) if c=='installed'}
def verify(wanted,kernel):
 assert os.geteuid()==0
 assert output(['uname','-r']).strip()==kernel
 assert installed()==wanted,'exact installed lock mismatch'
 assert not output(['dpkg','--audit']).strip(),'dpkg audit not empty'
 subprocess.run(['apt-get','--no-download','check'],check=True,stdout=subprocess.DEVNULL)
 import apt_pkg
 apt_pkg.init();cache=apt_pkg.Cache();deps=apt_pkg.DepCache(cache)
 assert deps.broken_count==0,'broken dependency count'
 return {'installed':installed(),'count':len(wanted),'kernel':kernel,'dpkg_audit':'empty','broken_dependencies':deps.broken_count}

def main():
 locks=json.loads(pathlib.Path(sys.argv[1]).read_text());mode=sys.argv[2];result={}
 if mode=='purge':
  result['before']=verify(locks['old'],locks['kernel'])
  result['held_before']=output(['apt-mark','showhold']).splitlines()
  plan=output(['apt-get','--simulate','--no-download','--allow-change-held-packages','purge',*sorted(locks['removed'])])
  purge_changes(plan,locks['removed']);result['simulation']=plan
  p=subprocess.run(['apt-get','-y','--no-download','--allow-change-held-packages','purge',*sorted(locks['removed'])],capture_output=True,text=True,timeout=600)
  result['transaction_stdout']=p.stdout;result['transaction_stderr']=p.stderr;result['transaction_returncode']=p.returncode
  if p.returncode:
   print(json.dumps(result));raise RuntimeError('purge transaction failed')
  result['held_after']=output(['apt-mark','showhold']).splitlines()
  assert set(result['held_after'])==set(result['held_before'])-set(locks['removed']),'unexpected hold changes'
 result['after']=verify(locks['new'],locks['kernel'])
 result['boot_current_kernel_present']=pathlib.Path('/boot/vmlinuz-'+locks['kernel']).is_file();assert result['boot_current_kernel_present']
 result['obsolete_boot_files']=[str(x) for x in pathlib.Path('/boot').glob('*7.0.11*')];assert not result['obsolete_boot_files']
 subprocess.run(['sync'],check=True);print(json.dumps(result))
if __name__=='__main__':main()

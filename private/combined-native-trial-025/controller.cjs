'use strict';
const fs=require('node:fs'),path=require('node:path'),{spawn}=require('node:child_process');
function markStopped(p){try{fs.writeFileSync(p,'',{flag:'wx'});}catch(e){if(e.code!=='EEXIST')throw e;}}
function safeLine(s){return /::add-mask::|signed_(?:upload|download)_url|[?&](?:sig|token|access_token|X-Amz-Signature)=|(?:ghp_|github_pat_)[A-Za-z0-9_]+/i.test(s)?'[REDACTED credential/capability-bearing output]\n':s;}
function sanitizer(write){let pending='',discard=false;return {feed(b){for(const ch of b.toString('utf8')){if(ch==='\n'){write(discard?'[REDACTED oversized output line]\n':safeLine(pending+'\n'));pending='';discard=false;}else if(!discard){pending+=ch;if(pending.length>65536){pending='';discard=true;}}}},end(){if(pending||discard)write(discard?'[REDACTED oversized output line]\n':safeLine(pending+'\n'));pending='';}};}
module.exports={markStopped,safeLine,sanitizer};
if(require.main===module){
 const work=process.env.WORK;fs.mkdirSync(path.join(work,'control'),{recursive:true});
 const log=fs.createWriteStream(path.join(work,'evidence/session.log'),{flags:'wx'});
 const filter=sanitizer(s=>{log.write(s);process.stdout.write(s);});
 const guest=spawn('sudo',['env','-i','PATH=/usr/sbin:/usr/bin:/sbin:/bin','WORK='+work,'LD_LIBRARY_PATH='+work+'/tools/usr/lib/x86_64-linux-gnu','PYTHONPATH='+work+'/tools/usr/lib/python3/dist-packages','PYTHONDONTWRITEBYTECODE=1','python3','-u','private/combined-native-trial-025/run_trial.py'],{stdio:['ignore','pipe','pipe']});
 guest.stdout.on('data',b=>filter.feed(b));guest.stderr.on('data',b=>filter.feed(b));
 for(const signal of ['SIGINT','SIGTERM'])process.on(signal,()=>guest.kill(signal));
 guest.on('error',()=>{filter.feed('controller child launch failed\n');process.exitCode=1;});
 guest.on('close',(code,signal)=>{filter.end();try{markStopped(path.join(work,'control/bridge-stop'));}catch(e){log.write('stop marker failure: '+e.code+'\n');process.exitCode=1;}log.end();process.exitCode=process.exitCode||((code===0&&!signal)?0:1);});
}

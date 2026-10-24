const {spawn, spawnSync}=require('node:child_process');
const fs=require('node:fs'); const path=require('node:path');
const work=process.env.WORK; const recipe='private/package-session-025/';
const mode=process.env.INPUT_MODE;
if(mode==='preflight'){
 const r=spawnSync('python3',['-u',recipe+'bridge.py','--preflight'],{stdio:'inherit',env:process.env,timeout:90000});
 process.exit(r.status===0?0:1);
}
if(mode!=='session')throw new Error('unknown controller mode');
fs.mkdirSync(path.join(work,'control'),{recursive:true});
const bridgeLog=fs.openSync(path.join(work,'evidence/bridge.log'),'wx');
const bridge=spawn('python3',['-u',recipe+'bridge.py'],{env:process.env,stdio:['ignore',bridgeLog,bridgeLog]});
let stopping=false; bridge.on('exit',code=>{if(!stopping)fs.writeFileSync(path.join(work,'control/bridge-failed'),String(code));});
const args=['env','-i','PATH=/usr/sbin:/usr/bin:/sbin:/bin','WORK='+work,'ACCEPTANCE_RUN_ID='+process.env.GITHUB_RUN_ID,'LD_LIBRARY_PATH='+work+'/tools/usr/lib/x86_64-linux-gnu','PYTHONPATH='+work+'/tools/usr/lib/python3/dist-packages','python3','-u',recipe+'run_session.py'];
const log=fs.createWriteStream(path.join(work,'evidence/session.log'),{flags:'wx'});
const guest=spawn('sudo',args,{stdio:['ignore','pipe','pipe']});
for(const stream of [guest.stdout,guest.stderr])stream.on('data',b=>{log.write(b);process.stdout.write(b);});
for(const signal of ['SIGINT','SIGTERM'])process.on(signal,()=>guest.kill(signal));
guest.on('exit',code=>{stopping=true;fs.writeFileSync(path.join(work,'control/bridge-stop'),'');bridge.kill('SIGTERM');log.end();process.exitCode=code===0?0:1;});

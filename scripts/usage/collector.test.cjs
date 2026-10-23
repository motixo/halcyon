const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const context = {module:{exports:{}}};
vm.runInNewContext(fs.readFileSync(__dirname+'/collector.gs','utf8'),context);
const {validateReport,updateRows}=context.module.exports;
const payload={schema:1,installation:'a'.repeat(64),version:'0.2.0'};
test('accepts only the small documented report',()=>{
 assert.equal(validateReport(JSON.stringify(payload)).version,'0.2.0');
 for (const invalid of [{...payload,email:'private@example.com'},{...payload,schema:2},{...payload,version:'=IMPORTXML("url")'},{...payload,installation:'short'},null,[]])
  assert.throws(()=>validateReport(JSON.stringify(invalid)));
 assert.throws(()=>validateReport(' '.repeat(1025)));
});
test('repeat reports keep one row and update the version',()=>{
 const rows=updateRows([], 'id-a','0.2.0',21000);
 assert.equal(rows.length,1);
 const twice=updateRows(rows,'id-a','0.1.3',21000);
 assert.equal(twice.length,1); assert.equal(twice[0][2],'0.1.3');
 const next=updateRows(twice,'id-a','0.1.3',21001);
 assert.equal(next[0][1],21001); assert.equal(next.length,1);
});
test('different installations remain distinct and inactive records expire',()=>{
 const rows=[['expired',20910,'0.2.0'],['retained',20911,'0.2.0']];
 const next=updateRows(rows,'new','0.2.0',21000);
 assert.equal(next.length,2); assert.equal(next[0][0],'retained');
 assert.equal(rows.length,2); assert.equal(rows[0][0],'expired');
});
test('existing identities can report at capacity, new ones cannot',()=>{
 const rows=Array.from({length:20000},(_,i)=>['id-'+i,21000,'0.2.0']);
 assert.equal(updateRows(rows,'id-0','0.1.3',21000).length,20000);
 assert.throws(()=>updateRows(rows,'new','0.2.0',21000));
});

function fixture() {
 const crypto=require('node:crypto');
 let rows=[['Installation hash','Last active (UTC)','Version']],maxRows=1,triggers=[],status='',locked=false,failWrite=false;
 const visible=()=>{let end=rows.length;while(end && rows[end-1].every(v=>v===''))end--;return rows.slice(0,end);};
 const raw={
  getMaxRows:()=>maxRows,
  insertRowsAfter:(_,count)=>{maxRows+=count;},
  setFrozenRows:()=>{},
  getLastRow:()=>visible().length,
  getRange:(start,col,count=1,width=1)=>{
   if (typeof start==='string') {
    assert.equal(start,'B2:B20001'); assert.ok(maxRows>=20001,'setup must expand an imported sheet');
    return {setNumberFormat:()=>{}};
   }
   if(start+count-1>maxRows) throw Error('Outside sheet');
   return {
    getValues:()=>rows.slice(start-1,start-1+count).map(r=>r.slice(col-1,col-1+width)),
    setValues:values=>{values.forEach((row,i)=>{rows[start-1+i]=row.slice();});if(failWrite){failWrite=false;throw Error('Interrupted after write');}},
    clearContent:()=>{rows.splice(start-1,count);},
   };
  },
 };
 let countryRows=[];
 const countries={getLastRow:()=>countryRows.length,getMaxRows:()=>1000,setFrozenRows:()=>{},
 getRange:()=>({setValues:values=>{countryRows=values.map(r=>r.slice());}})};
 const book={setSpreadsheetTimeZone:zone=>assert.equal(zone,'Etc/UTC'),getSheetByName:name=>name==='Installations'?raw:name==='Countries'?countries:{getRange:()=>({setValue:value=>{status=value;}})}};
 const trigger={timeBased:()=>trigger,everyDays:()=>trigger,atHour:()=>trigger,create:()=>triggers.push({getHandlerFunction:()=> 'maintenance'})};
 const sandbox={module:{exports:{}},Date,JSON,
  PropertiesService:{getScriptProperties:()=>({getProperty:()=> 'private-sheet-id'})},
  SpreadsheetApp:{openById:()=>book,flush:()=>{}},
  ScriptApp:{getProjectTriggers:()=>triggers,newTrigger:()=>trigger},
  LockService:{getScriptLock:()=>({tryLock:()=>{locked=true;return true;},hasLock:()=>locked,releaseLock:()=>{locked=false;}})},
  ContentService:{MimeType:{JSON:'application/json'},createTextOutput:text=>({setMimeType:()=>JSON.parse(text)})},
  Utilities:{DigestAlgorithm:{SHA_256:'sha256'},computeDigest:(_,value)=>[...crypto.createHash('sha256').update(value).digest()],formatDate:date=>date.toISOString().slice(0,10)},
 };
 vm.runInNewContext(fs.readFileSync(__dirname+'/collector.gs','utf8'),sandbox);
 return {sandbox,countries:()=>countryRows,rows:visible,status:()=>status,triggers:()=>triggers,locked:()=>locked,seed:values=>{rows=[rows[0],...values];},failAfterWrite:()=>{failWrite=true;}};
}
test('setup prepares imported small sheets and creates only one retention trigger',()=>{
 const f=fixture();
 assert.equal(f.sandbox.setup().configured,true);
 assert.equal(f.sandbox.setup().configured,true);
 assert.equal(f.status(),'Configured'); assert.equal(f.triggers().length,1);
});
test('real request handler stores a hash, date, version and optional country and deduplicates reports',()=>{
 const f=fixture();f.sandbox.setup();
 const request={postData:{contents:JSON.stringify(payload)}};
 const ack=f.sandbox.doPost(request);
 assert.equal(ack.ok,true);assert.equal(ack.day,new Date().toISOString().slice(0,10));
 assert.equal(f.sandbox.doPost(request).ok,true);
 assert.equal(f.rows().length,2); assert.equal(f.rows()[1].length,4);
 assert.match(f.rows()[1][0],/^[a-f0-9]{64}$/);assert.notEqual(f.rows()[1][0],payload.installation);
 assert.equal(f.rows()[1][2],payload.version);assert.equal(f.locked(),false);
 assert.equal(f.sandbox.doPost({postData:{contents:JSON.stringify({...payload,email:'private@example.com'})}}).ok,false);
 assert.equal(f.rows().length,2);
 assert.deepEqual(f.sandbox.doGet(),{service:'halcyon-activity',schema:2});
});

test('retry after an interrupted pruning write counts each installation once',()=>{
 const f=fixture();f.sandbox.setup();
 const today=Math.floor(Date.now()/86400000)+25569;
 f.seed([['e'.repeat(64),today-100,'0.2.0'],['f'.repeat(64),today-100,'0.2.0'],
         ['a'.repeat(64),today,'0.2.0'],['b'.repeat(64),today,'0.2.0']]);
 f.failAfterWrite();
 const request={postData:{contents:JSON.stringify(payload)}};
 assert.equal(f.sandbox.doPost(request).ok,false);
 assert.equal(f.sandbox.doPost(request).ok,true);
 assert.equal(f.rows().length,4); // header plus three distinct installations
 assert.equal(new Set(f.rows().slice(1).map(row=>row[0])).size,3);
});
test('maintenance repairs duplicate records and preserves the newest date and version',()=>{
 const f=fixture();f.sandbox.setup();
 const today=Math.floor(Date.now()/86400000)+25569;
 f.seed([['a',today-3,'0.1.1'],['a',today-1,'0.2.0'],['b',today-5,'0.1.1'],['expired',today-90,'0.1.1']]);
 f.sandbox.maintenance();
 assert.equal(f.rows().length,3);
 assert.equal(f.rows()[1][1],today-1);assert.equal(f.rows()[1][2],'0.2.0');
});

test('country reports accept schema 2 and reject unbounded or extra location data',()=>{
 const report={...payload,schema:2,country:'HR'};
 assert.equal(validateReport(JSON.stringify(report)).country,'HR');
 assert.equal(validateReport(JSON.stringify({...report,country:null})).country,null);
 for(const country of ['hr','H','HRV','1A','=US',{},42])
  assert.throws(()=>validateReport(JSON.stringify({...report,country})));
 assert.throws(()=>validateReport(JSON.stringify({...report,ip:'192.0.2.1'})));
});
test('first country survives later reports and old clients cannot erase it',()=>{
 let rows=updateRows([], 'country-id','0.2.0',21000,'HR');
 assert.equal(rows[0][3],'HR');
 rows=updateRows(rows,'country-id','0.2.0',21001,'US');
 assert.equal(rows[0][3],'HR');
 assert.equal(updateRows(rows,'country-id','0.2.0',21002)[0][3],'HR');
 assert.equal(updateRows([['unknown',21000,'0.2.0']], 'unknown','0.2.0',21001,'DE')[0][3],'DE');
});
test('country totals count distinct active installations in UTC windows, including Unknown',()=>{
 assert.equal(typeof context.countryCounts,'function');
 const rows=[['a',21000,'0.2.0','HR'],['a',20999,'0.1.1','HR'],['b',20994,'0.2.0','HR'],
 ['c',20971,'0.2.0','US'],['d',21000,'0.2.0',''],['old',20970,'0.1.1','DE']];
 assert.equal(JSON.stringify(context.countryCounts(rows,21000)),JSON.stringify([
 ['HR',1,2,2],['Unknown',1,1,1],['US',0,0,1]]));
});

test('request handlers migrate old rows and refresh country counts after retention',()=>{
 const f=fixture();f.sandbox.setup();
 const today=Math.floor(Date.now()/86400000)+25569;
 f.seed([['legacy',today,'0.1.1'],['expired',today-90,'0.2.0','US']]);
 const request={postData:{contents:JSON.stringify({...payload,schema:2,country:'HR'})}};
 assert.equal(f.sandbox.doPost(request).ok,true);
 assert.equal(f.rows()[1][3],'');
 assert.equal(f.rows()[2][3],'HR');
 assert.equal(JSON.stringify(f.countries()),JSON.stringify([
 ['First reported country','Active '+new Date().toISOString().slice(0,10)+' (UTC)','Last 7 days','Last 30 days'],['HR',1,1,1],['Unknown',1,1,1]]));
 f.seed([['expired',today-90,'0.2.0','US']]);f.sandbox.maintenance();
 assert.ok(f.countries().slice(1).every(row=>row.every(value=>value==='')));
});

const test=require('node:test');
const assert=require('node:assert/strict');
const {fixture}=require('./collector.fixture.cjs');
const token='a'.repeat(64);
const activity={schema:4,kind:'activity',installation:token,revision:0,version:'0.3.0',country:'HR',first_started_at:1792756800};
const now=new Date('2026-10-24T12:00:00Z');
test('schema 4 bounds startup timestamps and keeps status reports minimal',()=>{
 const {sandbox:s}=fixture();
 for(const first_started_at of [null,0,1792756800,253402300799])
  assert.equal(s.validateReport(JSON.stringify({...activity,first_started_at})).schema,4);
 for(const value of [-1,1.5,'2026-10-23',{},253402300800])
  assert.throws(()=>s.validateReport(JSON.stringify({...activity,first_started_at:value})));
 assert.throws(()=>s.validateReport(JSON.stringify({...activity,hostname:'private'})));
 assert.throws(()=>s.validateReport(JSON.stringify({schema:3,kind:'status',installation:token,revision:1,enabled:false,changed_at:0,first_started_at:0})));
});
test('first use and server observation stay distinct across offline delay, repeats and upgrades',()=>{
 const {sandbox:s}=fixture();
 let rows=s.applyReport([],token,activity,now);
 assert.equal(rows[0][9],now.toISOString());
 assert.equal(rows[0][10],new Date(activity.first_started_at*1000).toISOString());
 assert.equal(rows[0][11],'0.3.0');assert.equal(rows[0][12],1);
 rows=s.applyReport(rows,token,{...activity,version:'0.3.1',first_started_at:0},new Date(+now+3600000));
 assert.equal(rows[0][9],now.toISOString());assert.equal(rows[0][11],'0.3.0');
 assert.equal(rows[0][10],new Date(activity.first_started_at*1000).toISOString());
 assert.equal(rows[0][2],'0.3.1');assert.equal(rows[0][12],1);
 assert.equal(rows[0][14],new Date(+now+3600000).toISOString());
 rows=s.applyReport(rows,token,activity,new Date(+now+86400000));
 assert.equal(rows[0][12],2);
});
test('legacy migration leaves dates unknown and never fabricates old daily history',()=>{
 const f=fixture(),s=f.sandbox,today=s.dayNumber(now);
 f.seed([['old',today-1,'0.2.0','HR']]);f.sandbox.setup();
 assert.ok(f.rows()[1].slice(9).every(v=>v===''));
 const rows=s.applyReport(f.rows().slice(1),'old',{schema:2,version:'0.2.0',country:'HR'},now);
 assert.equal(rows[0][9],now.toISOString());assert.equal(rows[0][10],'');
 assert.equal(rows[0][12],1);
});
test('day history is bounded, duplicate-safe and expires on a quiet day',()=>{
 const {sandbox:s}=fixture();let rows=[];
 for(let day=0;day<120;day++) {
  const date=new Date(+now+day*86400000);
  rows=s.applyReport(rows,token,activity,date);
  rows=s.applyReport(rows,token,activity,date);
 }
 assert.equal(rows[0][12],90);assert.equal(JSON.parse(rows[0][13]).length,90);
 const merged=s.currentRows([...rows,...rows],s.dayNumber(new Date(+now+120*86400000)));
 assert.equal(merged.length,1);assert.equal(merged[0][12],89);
 assert.equal(s.currentRows(rows,s.dayNumber(new Date(+now+209*86400000))).length,0);
});
test('opt-out and stale schema 4 activity never add timing or activity details',()=>{
 const {sandbox:s}=fixture(),off={schema:3,kind:'status',revision:1,enabled:false,changed_at:activity.first_started_at};
 const rows=s.applyReport([],token,off,now);
 assert.ok(rows[0].slice(9).every(v=>v===''));
 assert.equal(JSON.stringify(s.applyReport(rows,token,activity,now)),JSON.stringify(rows));
 const enabled=s.applyReport(rows,token,{...off,revision:2,enabled:true},now);
 assert.equal(JSON.stringify(s.applyReport(enabled,token,activity,now)),JSON.stringify(enabled));
});
test('new metadata migration refuses occupied columns, formulas and malformed records before writes',()=>{
 for(let col=10;col<=15;col++) for(const [value,formula] of [[42,''],['','=""']]) {
  const f=fixture();f.raw.maxColumns=26;f.raw.maxRows=100;
  f.raw.put(40,col,value,formula);
  const before=f.snapshot();assert.throws(()=>f.sandbox.setup());assert.equal(f.snapshot(),before);
 }
 const f=fixture();f.sandbox.setup();f.seed([['id',46000,'0.3.0','','enabled','','',0,'','','','',1,'[46000,46000]','']]);
 const before=f.snapshot();assert.throws(()=>f.sandbox.maintenance());assert.equal(f.snapshot(),before);
});
test('interrupted activity writes retry without duplicate day counts or changed first dates',()=>{
 for(let cell=1;cell<=15;cell++) {
  const f=fixture();f.sandbox.setup();f.failAfterCells(cell);
  const request={postData:{contents:JSON.stringify(activity)}};
  assert.equal(f.sandbox.doPost(request).ok,false);
  assert.equal(f.sandbox.doPost(request).ok,true);
  assert.equal(f.rows().length,2);assert.equal(f.rows()[1][12],1);
  assert.equal(JSON.parse(f.rows()[1][13]).length,1);
 }
});

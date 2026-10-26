const test=require('node:test');
const assert=require('node:assert/strict');
const crypto=require('node:crypto');
const {fixture}=require('./collector.fixture.cjs');
const token='a'.repeat(64),id=crypto.createHash('sha256').update(token).digest('hex');
const today=()=>Math.floor(Date.now()/86400000)+25569;
const headers=['Reporting status','Opted out at (UTC)','Status received at (UTC)','Preference revision','Choice at (UTC)'];
const off=revision=>({postData:{contents:JSON.stringify({schema:3,kind:'status',installation:token,revision,enabled:false,changed_at:1792756800})}});
const legacy=()=>[id,today(),'0.2.0','HR'];
function knownHeaders(f) {
 f.raw.maxColumns=Math.max(f.raw.maxColumns,9);
 headers.forEach((v,i)=>f.raw.put(1,5+i,v));
}
function unchangedFailure(f,operation) {
 const before=f.snapshot(),writes=f.state.writes;
 if(operation==='post') assert.equal(f.sandbox.doPost(off(1)).ok,false);
 else assert.throws(()=>f.sandbox[operation]());
 assert.equal(f.snapshot(),before,`${operation} must preserve every workbook cell, formula, dimension and trigger`);
 assert.equal(f.state.writes,writes,`${operation} must preflight before its first write`);
 assert.equal(f.locked(),false);
}

test('U1: custom Reporting is preserved by setup, ingestion and maintenance before any writes',()=>{
 for(const operation of ['setup','post','maintenance']) {
  const f=fixture();f.seed([legacy()]);
  if(operation!=='setup') f.sandbox.setup();
  const historical=f.add('Reporting');
  historical.put(1,1,'Historical totals');historical.put(2,2,37,'=SUM(Historical!B2:B30)');
  unchangedFailure(f,operation);
  assert.equal(historical.cell(2,2).formula,'=SUM(Historical!B2:B30)');
 }
});
test('U2: blank H header with formula 42 never becomes an authoritative revision',()=>{
 const f=fixture();f.seed([legacy()]);f.raw.maxColumns=26;
 f.raw.put(2,8,42,'=SUM(Historical!B2:B30)');
 unchangedFailure(f,'setup');
 assert.equal(f.raw.cell(1,8).value,'');assert.equal(f.raw.cell(2,8).formula,'=SUM(Historical!B2:B30)');
 // A separate genuinely empty extension accepts the first real opt-out.
 const clean=fixture();clean.seed([legacy()]);clean.sandbox.setup();
 const ack=clean.sandbox.doPost(off(1));
 assert.equal(ack.ok,true);assert.equal(ack.revision,1);assert.equal(ack.enabled,false);
 assert.equal(clean.rows()[1][4],'opted_out');assert.equal(clean.rows()[1][7],1);
});
test('U2: every proposed status column rejects unrelated literals and formulas, including empty results and headers',()=>{
 for(let column=5;column<=9;column++) for(const [value,formula] of [[42,''],[false,''],['','=""'],[42,'=SUM(Historical!B2:B30)']]) {
  const f=fixture();f.seed([legacy()]);f.raw.maxColumns=26;f.raw.put(2,column,value,formula);
  unchangedFailure(f,'setup');
 }
 for(let column=5;column<=9;column++) {
  const f=fixture();f.seed([legacy()]);knownHeaders(f);
  f.raw.put(1,column,headers[column-5],'="'+headers[column-5]+'"');unchangedFailure(f,'setup');
 }
 // A formula on a later otherwise empty row must also be detected.
 const f=fixture();f.seed([legacy()]);f.raw.maxColumns=26;f.raw.maxRows=200;
 f.raw.put(150,9,'','=""');unchangedFailure(f,'setup');
});
test('U2: known headings do not authorize formulas, incoherent state or orphan status cells',()=>{
 for(const status of [
  ['enabled','','',42,'unrelated'],['opted_out','','',1,''],['enabled','','','42',''],
  ['custom','','',0,''],['enabled','','',0,'2026-10-23T12:00:00.000Z'],
 ]) {
  const f=fixture();f.seed([[...legacy(),...status]]);knownHeaders(f);unchangedFailure(f,'setup');
 }
 const orphan=fixture();orphan.seed([['','','','','enabled','','',0,'']]);knownHeaders(orphan);unchangedFailure(orphan,'setup');
 for(const operation of ['setup','post','maintenance']) {
  const f=fixture();f.seed([legacy()]);f.sandbox.setup();
  f.raw.put(2,8,42,'=SUM(Historical!B2:B30)');unchangedFailure(f,operation);
 }
});
test('empty legacy extension and headed partial initialization migrate without rewriting original cells',()=>{
 for(const partial of [false,true]) {
  const f=fixture();f.seed([legacy(),['second',today(),'0.2.0','DE']]);
  f.raw.put(2,3,'0.2.0','="0.2.0"'); // existing original-column formula is preserved by setup
  if(partial) {knownHeaders(f);f.raw.put(2,5,'enabled');}
  const beforeValues=f.raw.getRange(1,1,3,Math.min(4,f.raw.getMaxColumns())).getValues();
  const beforeFormula=f.raw.cell(2,3).formula;
  f.sandbox.setup();f.sandbox.setup();
  assert.equal(JSON.stringify(f.raw.getRange(2,1,2,4).getValues()),JSON.stringify(beforeValues.slice(1)));
  assert.equal(f.raw.cell(2,3).formula,beforeFormula);assert.equal(f.rows()[1][7],0);assert.equal(f.rows()[2][7],0);
  assert.equal(f.state.timezone,'Europe/Zagreb');assert.equal(f.triggers().length,1);
  assert.equal(f.summary()[1][0],'=SUM(Historical!B2:B30)');
 }
});
test('coherent existing status survives mixed migration and repeated setup without resetting off or revision',()=>{
 const received=new Date(),action=new Date('2026-10-23T12:00:00.000Z');
 const records=[
  [...legacy(),'opted_out',action,received,7,action],
  ['on',today(),'0.2.0','DE','enabled',action,received,8,action],
  ['aged',today(),'0.2.0','US','enabled','','',42,''],
  ['uninitialized',today(),'0.2.0',''],
 ];
 const f=fixture();f.seed(records);knownHeaders(f);
 const states=f.raw.getRange(2,5,3,5).getValues();
 f.sandbox.setup();f.sandbox.setup();
 assert.equal(JSON.stringify(f.raw.getRange(2,5,3,5).getValues()),JSON.stringify(states));
 assert.equal(f.sandbox.doPost(off(1)).revision,7);assert.equal(f.rows()[1][4],'opted_out');
 assert.equal(f.sandbox.doPost(off(8)).revision,8);assert.equal(f.rows()[1][4],'opted_out');
});
test('interruption through ownership, header and default-state cell writes is recoverable without row loss',()=>{
 for(let cells=1;cells<=37;cells++) {
  const f=fixture();const original=[legacy(),['other',today()-100,'0.1.1','DE']];f.seed(original);
  f.failAfterCells(cells);assert.throws(()=>f.sandbox.setup(),/Interrupted/);
  assert.equal(f.sandbox.setup().configured,true,`retry after ${cells} cells`);
  assert.equal(f.sandbox.setup().configured,true);
  assert.equal(JSON.stringify(f.rows().slice(1).map(r=>r.slice(0,4))),JSON.stringify(original));
  assert.equal(f.rows()[1][7],0);assert.equal(f.rows()[2][7],0);assert.equal(f.triggers().length,1);
  assert.equal(f.sandbox.doPost(off(1)).revision,1);
 }
});
test('Reporting ownership is stable, formula-sensitive and preserves cells outside the reserved snapshot',()=>{
 const f=fixture();f.seed([legacy()]);const empty=f.add('Reporting',1,1);
 f.sandbox.setup();const marker=empty.cell(8,1).value;assert.match(marker,/halcyon/);
 empty.insertRowsAfter(empty.getMaxRows(),20-empty.getMaxRows());
 empty.insertColumnsAfter(empty.getMaxColumns(),3-empty.getMaxColumns());
 empty.put(20,3,37,'=SUM(Historical!B2:B30)');
 f.sandbox.setup();f.sandbox.maintenance();assert.equal(f.sandbox.doPost(off(1)).ok,true);
 assert.equal(empty.cell(20,3).formula,'=SUM(Historical!B2:B30)');
 assert.equal(empty.cell(8,1).value,marker);
 empty.put(8,1,marker,'="'+marker+'"');unchangedFailure(f,'setup');
 const blank=fixture();const tab=blank.add('Reporting');tab.put(50,1,'','=""');unchangedFailure(blank,'setup');
});

test('a claimed Reporting snapshot refuses custom formulas or literals inside its reserved cells',()=>{
 for(const [r,c,value,formula] of [[1,1,'Historical totals',''],[2,2,37,'=SUM(Historical!B2:B30)'],[6,2,'Owner note','']]) {
  for(const operation of ['setup','post','maintenance']) {
   const f=fixture();f.seed([legacy()]);f.sandbox.setup();f.sheets.get('Reporting').put(r,c,value,formula);
   unchangedFailure(f,operation);
  }
 }
});

test('native Sheets date coercion keeps the owned reporting snapshot usable',()=>{
 for(const operation of ['setup','post','maintenance']) {
  const f=fixture();f.seed([legacy()]);f.sandbox.setup();
  const tab=f.sheets.get('Reporting');
  tab.put(1,2,new Date(new Date().toISOString().slice(0,10)+'T00:00:00Z'));
  if(operation==='post') assert.equal(f.sandbox.doPost(off(1)).ok,true);
  else f.sandbox[operation]();
  tab.put(1,2,new Date(NaN));unchangedFailure(f,operation);
 }
});

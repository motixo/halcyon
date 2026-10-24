/* Halcyon collector. Deploy as owner, ANYONE_ANONYMOUS; Sheet stays private.
 * SPREADSHEET_ID remains in Script Properties. Never log bodies or identities.
 */
const MAX_INSTALLATIONS = 20000;
const RETENTION_DAYS = 90;
const WIDTH = 15;
const HEADERS = ['Installation hash','Last active (UTC)','Version','First reported country',
  'Reporting status','Opted out at (UTC)','Status received at (UTC)','Preference revision','Choice at (UTC)',
  'First activity received since tracking (UTC)','First desktop start (client UTC; if known)',
  'First version received since tracking','Reported active days (last 90 days)',
  'Activity days (UTC serials; internal)','Last activity received (UTC)'];
function validCountry(value) { return typeof value === 'string' && /^[A-Z]{2}$/.test(value); }
function validVersion(v) { return typeof v === 'string' && v.length <= 48 && /^\d+\.\d+\.\d+(?:-[a-zA-Z0-9.-]+)?$/.test(v); }
function validTime(t) { return Number.isSafeInteger(t) && t >= 0 && t <= 253402300799; }
function iso(value) {
  if (value === '' || value === undefined) return '';
  const date=value instanceof Date ? value : typeof value === 'string' ? new Date(value) : null;
  if (!date || !Number.isFinite(date.getTime()) || date.getTime()<0 || date.getTime()>253402300799999) throw Error('Invalid timestamp');
  const result=date.toISOString();
  if (!(value instanceof Date) && value!==result) throw Error('Invalid timestamp');
  return result;
}
function days_(value) {
  if (value === '' || value === undefined) return [];
  // Brackets keep a single day from being treated as a numeric Sheet cell.
  if (typeof value !== 'string' || value.length>800 || !/^\[\d{1,7}(,\d{1,7})*\]$/.test(value)) throw Error('Invalid activity days');
  const days=JSON.parse(value);
  if (days.length>RETENTION_DAYS || days.some((d,i)=>d<25569 || d>2958465 || i>0 && d<=days[i-1])) throw Error('Invalid activity days');
  return days;
}
function refreshDays_(row,today,extra=[]) {
  const days=[...new Set([...days_(row[13]),...extra])].filter(d=>d>=today-89 && d<=today).sort((a,b)=>a-b);
  row[13]=days.length ? JSON.stringify(days) : ''; row[12]=days.length || '';
}
function validateReport(body) {
  if (typeof body !== 'string' || body.length > 1024) throw Error('Invalid report');
  const v = JSON.parse(body);
  if (!v || Array.isArray(v) || typeof v.installation !== 'string' || !/^[a-f0-9]{64}$/.test(v.installation)) throw Error('Invalid report');
  const keys = Object.keys(v).sort().join(',');
  const legacy = v.schema === 1 && keys === 'installation,schema,version' && validVersion(v.version)
    || v.schema === 2 && keys === 'country,installation,schema,version' && validVersion(v.version) && (v.country === null || validCountry(v.country));
  const revision = Number.isSafeInteger(v.revision) && v.revision >= 0;
  const activity = v.schema === 3 && v.kind === 'activity' && keys === 'country,installation,kind,revision,schema,version'
    && revision && validVersion(v.version) && (v.country === null || validCountry(v.country));
  const detailed = v.schema === 4 && v.kind === 'activity' && keys === 'country,first_started_at,installation,kind,revision,schema,version'
    && revision && validVersion(v.version) && (v.country === null || validCountry(v.country))
    && (v.first_started_at === null || validTime(v.first_started_at));
  // Client action time is informational, never a receipt/retention clock. Accept
  // skew within the representable UTC range; revision alone orders choices.
  const status = v.schema === 3 && v.kind === 'status' && keys === 'changed_at,enabled,installation,kind,revision,schema'
    && revision && v.revision > 0 && typeof v.enabled === 'boolean'
    && Number.isSafeInteger(v.changed_at) && v.changed_at >= 0 && v.changed_at <= 253402300799;
  if (!(legacy || activity || detailed || status)) throw Error('Invalid report');
  return v;
}
function dayNumber(date) { return Math.floor(date.getTime() / 86400000) + 25569; }
function time(value) { return value ? new Date(value).getTime() : NaN; }
function normalize(row) {
  const active = row[1] instanceof Date ? dayNumber(row[1]) : (row[1] === '' ? '' : Number(row[1]));
  return [row[0], active, row[2] || '', validCountry(row[3]) ? row[3] : '',
    row[4] || 'enabled', row[5] instanceof Date ? row[5].toISOString() : row[5] || '',
    row[6] instanceof Date ? row[6].toISOString() : row[6] || '', Number(row[7] || 0),
    row[8] instanceof Date ? row[8].toISOString() : row[8] || '',
    iso(row[9]),iso(row[10]),row[11] || '',row[12] || '',row[13] || '',iso(row[14])];
}
function currentRows(rows, today) {
  const unique = new Map();
  for (const raw of rows) {
    if (!raw[0]) continue;
    const row = normalize(raw), previous = unique.get(row[0]);
    if (!previous) { unique.set(row[0], row); continue; }
    // A repeated/partially compacted row must not inflate active-day counts or
    // replace the earliest observation with the date of its retry.
    const days=[...days_(row[13]),...days_(previous[13])];
    const first = !previous[9] || row[9] && row[9]<previous[9] ? row : previous;
    const started=[row[10],previous[10]].filter(Boolean).sort()[0] || '';
    const latest=[row[14],previous[14]].filter(Boolean).sort().pop() || '';
    const metadata=[first[9],started,first[11],'','',latest];
    // A committed higher revision is authoritative even if a failed compaction
    // left an older row later in the sheet. Equal-revision opt-out wins closed.
    const stronger = row[7] > previous[7] || row[7] === previous[7] && row[4] === 'opted_out' && previous[4] !== 'opted_out';
    if (stronger || row[7] === previous[7] && row[4] === previous[4] && Number(row[1]) > Number(previous[1])) {
      row[3] = previous[3] || row[3]; unique.set(row[0], row);
    } else previous[3] ||= row[3];
    const selected=unique.get(row[0]); selected.splice(9,6,...metadata); refreshDays_(selected,today,days);
  }
  return Array.from(unique.values()).filter(row => {
    refreshDays_(row,today);
    const receivedDay = Number.isFinite(time(row[6])) ? dayNumber(new Date(row[6])) : 0;
    // Opted-out rows expire from status receipt, never from action time or an
    // old client's replay. Enabled records also retain their recent activity.
    const retained = row[4] === 'opted_out' ? receivedDay : Math.max(Number(row[1]) || 0, receivedDay);
    if (retained < today - RETENTION_DAYS + 1) return false;
    if (row[4] !== 'opted_out' && receivedDay < today - RETENTION_DAYS + 1) {
      row[5] = ''; row[6] = ''; row[8] = ''; // age out identifiable choice timestamps
    }
    return true;
  });
}
function applyReport(rows, id, report, now) {
  const today = dayNumber(now), next = currentRows(rows, today);
  let row = next.find(r => r[0] === id);
  if (report.kind === 'status') {
    if (row && report.revision < row[7]) return next;
    if (row && report.revision === row[7]) {
      // Exact retries are no-ops: neither receipt nor retention is refreshed.
      if (row[4] !== (report.enabled ? 'enabled' : 'opted_out') || row[8] && row[8] !== new Date(report.changed_at*1000).toISOString()) throw Error('Revision conflict');
      return next;
    }
    if (!row) {
      if (next.length >= MAX_INSTALLATIONS) throw Error('Capacity reached');
      row = normalize([id,'','','','enabled','','',0,'']); next.push(row);
    }
    row[4] = report.enabled ? 'enabled' : 'opted_out';
    row[8] = new Date(report.changed_at * 1000).toISOString();
    if (!report.enabled) row[5] = row[8];
    row[6] = now.toISOString(); row[7] = report.revision;
    return next; // status never touches active date, country or app version
  }
  if (row && (row[4] !== 'enabled' || report.schema >= 3 && report.revision !== row[7])) return next;
  // Old clients may continue enabled activity but never change preference state.
  if (!row) {
    if (next.length >= MAX_INSTALLATIONS) throw Error('Capacity reached');
    row = normalize([id,'','','','enabled','','',report.revision || 0,'']); next.push(row);
  }
  row[1] = today; row[2] = report.version;
  row[3] ||= validCountry(report.country) ? report.country : '';
  if (!row[9]) { row[9]=now.toISOString(); row[11]=report.version; }
  if (!row[10] && report.schema === 4 && report.first_started_at !== null)
    row[10]=new Date(report.first_started_at*1000).toISOString();
  row[14]=now.toISOString(); refreshDays_(row,today,[today]);
  return next;
}
function updateRows(rows, id, version, today, country = null) {
  return applyReport(rows, id, {schema:2,version,country}, new Date((today-25569)*86400000));
}
function countryCounts(rows, today) {
  const countries = new Map();
  for (const row of currentRows(rows, today)) {
    if (row[4] !== 'enabled' || row[1] === '' || row[1] > today || row[1] < today-29) continue;
    const country = row[3] || 'Unknown';
    if (!countries.has(country)) countries.set(country, [country,0,0,0]);
    const counts = countries.get(country);
    if (row[1] === today) counts[1]++;
    if (row[1] >= today-6) counts[2]++;
    counts[3]++;
  }
  return Array.from(countries.values()).sort((a,b)=>b[3]-a[3] || a[0].localeCompare(b[0]));
}
function countries_(book, rows, today) {
  const sheet = book.getSheetByName('Countries') || book.insertSheet('Countries');
  // Label the snapshot date so a quiet period or delayed trigger cannot look current.
  const date = new Date((today-25569)*86400000).toISOString().slice(0,10);
  const values = [['First reported country', 'Active '+date+' (UTC)', 'Last 7 days', 'Last 30 days'],
    ...countryCounts(rows, today)];
  const count = Math.max(values.length, sheet.getLastRow());
  if (sheet.getMaxRows() < count) sheet.insertRowsAfter(sheet.getMaxRows(), count-sheet.getMaxRows());
  while (values.length < count) values.push(['','','','']);
  sheet.getRange(1,1,count,4).setValues(values);
  sheet.setFrozenRows(1);
}
function book_() {
  const id = PropertiesService.getScriptProperties().getProperty('SPREADSHEET_ID');
  if (!id) throw new Error('Collector not configured');
  return SpreadsheetApp.openById(id);
}
// A header alone is not permission to consume owner data. Read both values
// and formulas before any migration/ingestion/maintenance write. Missing headers
// may only cover empty columns; recognized headings require coherent state.
function statusPlan_(sheet, requireHeaders = false) {
  const count = Math.max(1,sheet.getLastRow()), width = Math.min(WIDTH,sheet.getMaxColumns());
  if (count > MAX_INSTALLATIONS+1) throw Error('Capacity reached');
  const range=sheet.getRange(1,1,count,width);
  const pad=row=>Array.from({length:WIDTH},(_,i)=>row[i] ?? '');
  const values=range.getValues().map(pad),formulas=range.getFormulas().map(pad);
  for (let col=4;col<WIDTH;col++) {
    if (formulas.some(row=>row[col] !== '')) throw Error('Formula in reporting status column');
    const header=values[0][col];
    if (header !== HEADERS[col] && header !== '') throw Error('Unrecognized existing status column');
    if (header === '' && (requireHeaders || values.slice(1).some(row=>row[col] !== '')))
      throw Error('Occupied or unmigrated reporting status column');
  }
  const initialize=[];
  for (let i=1;i<values.length;i++) {
    const row=values[i],v=row.slice(4,9),empty=row.slice(4).every(value=>value === '');
    // Added metadata columns are optional for migrated rows; never backdate
    // observation or infer a first start/version from the old last-active cell.
    [9,10,14].forEach(col=>iso(row[col]));
    if (row[11]!=='' && !validVersion(row[11])) throw Error('Invalid first version');
    days_(row[13]);
    if (row[12]!=='' && (!Number.isSafeInteger(row[12]) || row[12]<0 || row[12]>RETENTION_DAYS)) throw Error('Invalid active-day count');
    if (!row[0]) {
      if (!empty) throw Error('Reporting status without an installation');
      continue;
    }
    // A prefix of the default-state write is safe to finish. Never infer an
    // off status, timestamp or positive revision from incomplete cells.
    if ((v[0] === '' || v[0] === 'enabled') && (v[3] === '' || v[3] === 0)
        && v[1] === '' && v[2] === '' && v[4] === '') {
      if (v[0] !== 'enabled' || v[3] !== 0) initialize.push([i+1,['enabled','','',0,'']]);
      continue;
    }
    const timestamp=value=>{
      if (value === '') return '';
      const date=value instanceof Date ? value : typeof value === 'string' ? new Date(value) : null;
      if (!date || !Number.isFinite(date.getTime()) || date.getTime()<0 || date.getTime()>253402300799999) return null;
      const iso=date.toISOString();
      return value instanceof Date || value === iso ? iso : null;
    };
    const opted=timestamp(v[1]),received=timestamp(v[2]),choice=timestamp(v[4]),revision=v[3];
    if (!['enabled','opted_out'].includes(v[0]) || !Number.isSafeInteger(revision) || revision<=0
        || [opted,received,choice].includes(null) || Boolean(received)!==Boolean(choice)
        || (!received && opted) || (v[0] === 'opted_out' && (!received || !opted || opted!==choice)))
      throw Error('Incoherent reporting status');
  }
  return {values,formulas,initialize,rows:values.slice(1).map(normalize)};
}
function rows_(sheet) { return statusPlan_(sheet,true).rows; }
const REPORTING_OWNER = 'halcyon:reporting:schema3:v1';
const REPORTING_LABELS = ['Reporting snapshot (UTC)','Active today (reporting enabled)',
  'Active last 7 days (reporting enabled)','Active last 30 days (reporting enabled)',
  'Currently opted out (status received in last 90 days)','Counts are retained installations, not lifetime users'];
// A8 is outside the A1:B6 snapshot. An unmarked nonempty tab is never adopted,
// even if its labels happen to resemble our snapshot or formulas display blank.
function reporting_(book) {
  const sheet=book.getSheetByName('Reporting');
  if (!sheet) return null;
  if (sheet.getMaxRows()>=8) {
    const marker=sheet.getRange(8,1,1,1);
    if (marker.getValues()[0][0] === REPORTING_OWNER && marker.getFormulas()[0][0] === '') {
      if (sheet.getMaxColumns()<2) throw Error('Invalid Reporting snapshot');
      const snapshot=sheet.getRange(1,1,6,2);
      if (snapshot.getFormulas().some(row=>row.some(Boolean))) throw Error('Formula in Reporting snapshot');
      snapshot.getValues().forEach(([label,value],i)=>{
        // Accept empty prefixes left by interruption, but not owner edits inside
        // the reserved snapshot. A copied marker alone cannot authorize them.
        const validValue=value === '' || (i === 0 ? typeof value === 'string' && /^\d{4}-\d{2}-\d{2}$/.test(value)
          : i<5 && Number.isSafeInteger(value) && value>=0);
        if ((label !== REPORTING_LABELS[i] && (label !== '' || value !== '')) || !validValue)
          throw Error('Unrecognized Reporting snapshot contents');
      });
      return sheet;
    }
  }
  if (sheet.getLastRow() !== 0) throw Error('Reporting destination is not collector-owned or empty');
  return sheet;
}
function claimReporting_(book) {
  const sheet=reporting_(book) || book.insertSheet('Reporting');
  if (sheet.getMaxRows()<8) sheet.insertRowsAfter(sheet.getMaxRows(),8-sheet.getMaxRows());
  if (sheet.getMaxColumns()<2) sheet.insertColumnsAfter(sheet.getMaxColumns(),2-sheet.getMaxColumns());
  const marker=sheet.getRange(8,1,1,1);
  if (marker.getValues()[0][0] !== REPORTING_OWNER) marker.setValue(REPORTING_OWNER);
  return sheet;
}
function write_(sheet, before, after) {
  if (after.length === before.length && after.every((row,i)=>row[0] === before[i][0])) {
    for (let i=0;i<after.length;i++) {
      if (JSON.stringify(after[i]) !== JSON.stringify(before[i])) sheet.getRange(i+2,1,1,WIDTH).setValues([after[i]]);
    }
  } else {
    if (sheet.getMaxRows() < after.length+1) sheet.insertRowsAfter(sheet.getMaxRows(),after.length+1-sheet.getMaxRows());
    const values = after.concat(Array.from({length:Math.max(0,before.length-after.length)},()=>Array(WIDTH).fill('')));
    if (values.length) sheet.getRange(2,1,values.length,WIDTH).setValues(values);
  }
  SpreadsheetApp.flush();
}
function summary_(book, rows, today) {
  // Dedicated current snapshot. Do not overwrite existing historical aggregates
  // or any other Summary cells/formulas/layout owned by the private workbook.
  const sheet = claimReporting_(book);
  const current = currentRows(rows,today), enabled = current.filter(r=>r[4] === 'enabled' && r[1] !== '' && r[1] <= today);
  const values=[new Date((today-25569)*86400000).toISOString().slice(0,10),
    enabled.filter(r=>r[1]===today).length,enabled.filter(r=>r[1]>=today-6).length,
    enabled.filter(r=>r[1]>=today-29).length,current.filter(r=>r[4]==='opted_out').length,''];
  sheet.getRange(1,1,6,2).setValues(REPORTING_LABELS.map((label,i)=>[label,values[i]]));
}
function json_(value) { return ContentService.createTextOutput(JSON.stringify(value)).setMimeType(ContentService.MimeType.JSON); }
function doGet() { return json_({service:'halcyon-activity',schema:4}); }
function doPost(event) {
  let lock;
  try {
    const report = validateReport(event && event.postData && event.postData.contents);
    const id = Utilities.computeDigest(Utilities.DigestAlgorithm.SHA_256,report.installation)
      .map(b=>('0'+((b+256)%256).toString(16)).slice(-2)).join('');
    lock = LockService.getScriptLock();
    if (!lock.tryLock(3000)) return json_({ok:false});
    const now=new Date(),today=dayNumber(now),book=book_(),sheet=book.getSheetByName('Installations');
    const before=rows_(sheet); reporting_(book); // read-only preflight before any write
    const after=applyReport(before,id,report,now);
    write_(sheet,before,after);
    countries_(book,after,today); summary_(book,after,today);
    const row=after.find(r=>r[0]===id);
    return json_(report.kind === 'status' ? {ok:true,revision:row[7],enabled:row[4]==='enabled'}
      : {ok:true,day:Utilities.formatDate(now,'UTC','yyyy-MM-dd')});
  } catch (_) { return json_({ok:false}); }
  finally { if (lock && lock.hasLock()) lock.releaseLock(); }
}
function maintenance() {
  const lock=LockService.getScriptLock(); if (!lock.tryLock(10000)) return;
  try {
    const book=book_(),sheet=book.getSheetByName('Installations'),before=rows_(sheet),today=dayNumber(new Date());
    reporting_(book); // collisions must leave even Installations untouched
    const after=currentRows(before,today); write_(sheet,before,after);
    countries_(book,after,today); summary_(book,after,today);
  } finally { lock.releaseLock(); }
}
// Upgrade only the three documented current-count formulas wherever the owner
// placed them. Leave custom formulas, formatting and historical values intact.
function migrateSummary_(sheet) {
  const formulas = sheet.getDataRange().getFormulas();
  const originals = [0,6,29].map(days => '=COUNTIFS(Installations!B2:B20001,">="&TODAY()'+(days ? '-'+days : '')+',Installations!B2:B20001,"<="&TODAY())');
  formulas.forEach((row,i)=>row.forEach((formula,j)=>{
    if (originals.includes(formula)) sheet.getRange(i+1,j+1).setFormula(formula.slice(0,-1)+',Installations!E2:E20001,"enabled")');
  }));
}
// Schema-only migration: append columns, preserve every original row and all
// workbook properties. No deletion, timezone change, sorting or pruning here.
function setup() {
  const lock=LockService.getScriptLock(); if (!lock.tryLock(10000)) throw Error('Busy');
  try {
    const book=book_(),raw=book.getSheetByName('Installations');
    const plan=statusPlan_(raw);
    reporting_(book); // finish both preflights before changing any workbook cell
    claimReporting_(book); // recoverable even if the next write is interrupted
    if (raw.getMaxColumns() < WIDTH) raw.insertColumnsAfter(raw.getMaxColumns(),WIDTH-raw.getMaxColumns());
    if (raw.getMaxRows() < MAX_INSTALLATIONS+1) raw.insertRowsAfter(raw.getMaxRows(),MAX_INSTALLATIONS+1-raw.getMaxRows());
    raw.getRange(1,5,1,WIDTH-4).setValues([HEADERS.slice(4)]);
    if (plan.values[0][3] === '' && plan.formulas[0][3] === '') raw.getRange(1,4).setValue(HEADERS[3]);
    // Initialize only empty/default prefixes. Valid status values (including
    // Date cells) and all original A:D cells remain exactly as they were.
    // Batch adjacent legacy rows instead of issuing one Sheets call per row.
    for (let i=0;i<plan.initialize.length;) {
      const row=plan.initialize[i][0],values=[];
      do { values.push(plan.initialize[i++][1]); }
      while (i<plan.initialize.length && plan.initialize[i][0] === row+values.length);
      raw.getRange(row,5,values.length,5).setValues(values);
    }
    const rows=plan.rows;
    SpreadsheetApp.flush();
    if (!ScriptApp.getProjectTriggers().some(t=>t.getHandlerFunction()==='maintenance'))
      ScriptApp.newTrigger('maintenance').timeBased().everyDays(1).atHour(3).create();
    countries_(book,rows,dayNumber(new Date())); summary_(book,rows,dayNumber(new Date()));
    migrateSummary_(book.getSheetByName('Summary'));
    book.getSheetByName('Summary').getRange('B9').setValue('Configured');
    return {configured:true};
  } finally { lock.releaseLock(); }
}
if (typeof module !== 'undefined') module.exports = {validateReport,updateRows,applyReport,currentRows};

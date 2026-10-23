/* Halcyon collector. Deploy as owner, ANYONE_ANONYMOUS; Sheet stays private.
 * SPREADSHEET_ID remains in Script Properties. Never log bodies or identities.
 */
const MAX_INSTALLATIONS = 20000;
const RETENTION_DAYS = 90;
const WIDTH = 9;
const HEADERS = ['Installation hash','Last active (UTC)','Version','First reported country',
  'Reporting status','Opted out at (UTC)','Status received at (UTC)','Preference revision','Choice at (UTC)'];
function validCountry(value) { return typeof value === 'string' && /^[A-Z]{2}$/.test(value); }
function validVersion(v) { return typeof v === 'string' && v.length <= 48 && /^\d+\.\d+\.\d+(?:-[a-zA-Z0-9.-]+)?$/.test(v); }
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
  // Client action time is informational, never a receipt/retention clock. Accept
  // skew within the representable UTC range; revision alone orders choices.
  const status = v.schema === 3 && v.kind === 'status' && keys === 'changed_at,enabled,installation,kind,revision,schema'
    && revision && v.revision > 0 && typeof v.enabled === 'boolean'
    && Number.isSafeInteger(v.changed_at) && v.changed_at >= 0 && v.changed_at <= 253402300799;
  if (!(legacy || activity || status)) throw Error('Invalid report');
  return v;
}
function dayNumber(date) { return Math.floor(date.getTime() / 86400000) + 25569; }
function time(value) { return value ? new Date(value).getTime() : NaN; }
function normalize(row) {
  const active = row[1] instanceof Date ? dayNumber(row[1]) : (row[1] === '' ? '' : Number(row[1]));
  return [row[0], active, row[2] || '', validCountry(row[3]) ? row[3] : '',
    row[4] || 'enabled', row[5] instanceof Date ? row[5].toISOString() : row[5] || '',
    row[6] instanceof Date ? row[6].toISOString() : row[6] || '', Number(row[7] || 0),
    row[8] instanceof Date ? row[8].toISOString() : row[8] || ''];
}
function currentRows(rows, today) {
  const unique = new Map();
  for (const raw of rows) {
    if (!raw[0]) continue;
    const row = normalize(raw), previous = unique.get(row[0]);
    if (!previous) { unique.set(row[0], row); continue; }
    // A committed higher revision is authoritative even if a failed compaction
    // left an older row later in the sheet. Equal-revision opt-out wins closed.
    const stronger = row[7] > previous[7] || row[7] === previous[7] && row[4] === 'opted_out' && previous[4] !== 'opted_out';
    if (stronger || row[7] === previous[7] && row[4] === previous[4] && Number(row[1]) > Number(previous[1])) {
      row[3] = previous[3] || row[3]; unique.set(row[0], row);
    } else previous[3] ||= row[3];
  }
  return Array.from(unique.values()).filter(row => {
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
      row = [id,'','','','enabled','','',0,'']; next.push(row);
    }
    row[4] = report.enabled ? 'enabled' : 'opted_out';
    row[8] = new Date(report.changed_at * 1000).toISOString();
    if (!report.enabled) row[5] = row[8];
    row[6] = now.toISOString(); row[7] = report.revision;
    return next; // status never touches active date, country or app version
  }
  if (row && (row[4] !== 'enabled' || report.schema === 3 && report.revision !== row[7])) return next;
  // Old clients may continue enabled activity but never change preference state.
  if (!row) {
    if (next.length >= MAX_INSTALLATIONS) throw Error('Capacity reached');
    row = [id,'','','','enabled','','',report.revision || 0,'']; next.push(row);
  }
  row[1] = today; row[2] = report.version;
  row[3] ||= validCountry(report.country) ? report.country : '';
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
function rows_(sheet) {
  const count = sheet.getLastRow()-1;
  if (count < 1) return [];
  if (count > MAX_INSTALLATIONS) throw Error('Capacity reached');
  return sheet.getRange(2,1,count,WIDTH).getValues().map(normalize);
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
  const sheet = book.getSheetByName('Reporting') || book.insertSheet('Reporting');
  const current = currentRows(rows,today), enabled = current.filter(r=>r[4] === 'enabled' && r[1] !== '' && r[1] <= today);
  sheet.getRange(1,1,6,2).setValues([
    ['Reporting snapshot (UTC)',new Date((today-25569)*86400000).toISOString().slice(0,10)],
    ['Active today (reporting enabled)',enabled.filter(r=>r[1]===today).length],
    ['Active last 7 days (reporting enabled)',enabled.filter(r=>r[1]>=today-6).length],
    ['Active last 30 days (reporting enabled)',enabled.filter(r=>r[1]>=today-29).length],
    ['Currently opted out (status received in last 90 days)',current.filter(r=>r[4]==='opted_out').length],
    ['Counts are retained installations, not lifetime users','']]);
}
function json_(value) { return ContentService.createTextOutput(JSON.stringify(value)).setMimeType(ContentService.MimeType.JSON); }
function doGet() { return json_({service:'halcyon-activity',schema:3}); }
function doPost(event) {
  let lock;
  try {
    const report = validateReport(event && event.postData && event.postData.contents);
    const id = Utilities.computeDigest(Utilities.DigestAlgorithm.SHA_256,report.installation)
      .map(b=>('0'+((b+256)%256).toString(16)).slice(-2)).join('');
    lock = LockService.getScriptLock();
    if (!lock.tryLock(3000)) return json_({ok:false});
    const now=new Date(),today=dayNumber(now),book=book_(),sheet=book.getSheetByName('Installations');
    const before=rows_(sheet),after=applyReport(before,id,report,now);
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
    const headers=raw.getRange(1,1,1,Math.min(raw.getMaxColumns(),WIDTH)).getValues()[0];
    for (let i=4;i<headers.length;i++) {
      if (headers[i] && headers[i] !== HEADERS[i]) throw Error('Unrecognized existing status column');
    }
    if (raw.getMaxColumns() < WIDTH) raw.insertColumnsAfter(raw.getMaxColumns(),WIDTH-raw.getMaxColumns());
    if (raw.getMaxRows() < MAX_INSTALLATIONS+1) raw.insertRowsAfter(raw.getMaxRows(),MAX_INSTALLATIONS+1-raw.getMaxRows());
    raw.getRange(1,1,1,WIDTH).setValues([HEADERS]);
    const rows=rows_(raw);
    // Only new cells are initialized: retain original dates/types/values in A:D.
    if (rows.length) raw.getRange(2,5,rows.length,5).setValues(rows.map(r=>r.slice(4)));
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

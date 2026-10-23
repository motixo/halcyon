/* Halcyon activity collector. Deploy as owner, access ANYONE_ANONYMOUS.
 * The spreadsheet itself must remain private. Never log request contents.
 * Configure SPREADSHEET_ID in Script Properties before deployment.
 */
const MAX_INSTALLATIONS = 20000;
const RETENTION_DAYS = 90;

function validateReport(body) {
  if (typeof body !== 'string' || body.length > 1024) throw new Error('Invalid report');
  const value = JSON.parse(body);
  if (!value || Array.isArray(value) || !((value.schema === 1 && Object.keys(value).sort().join(',') === 'installation,schema,version')
        || (value.schema === 2 && Object.keys(value).sort().join(',') === 'country,installation,schema,version'
          && (value.country === null || validCountry(value.country)))) || typeof value.installation !== 'string'
      || !/^[a-f0-9]{64}$/.test(value.installation)
      || typeof value.version !== 'string' || value.version.length > 48
      || !/^\d+\.\d+\.\d+(?:-[a-zA-Z0-9.-]+)?$/.test(value.version)) throw new Error('Invalid report');
  return value;
}
function validCountry(value) { return typeof value === 'string' && /^[A-Z]{2}$/.test(value); }
function dayNumber(date) { return Math.floor(date.getTime() / 86400000) + 25569; }
function currentRows(rows, today) {
  const unique = new Map();
  for (const row of rows) {
    const day = Number(row[1]);
    if (!row[0] || !Number.isFinite(day) || day < today - RETENTION_DAYS + 1) continue;
    const previous = unique.get(row[0]);
    const country = (previous && previous[3]) || (validCountry(row[3]) ? row[3] : '');
    if (!previous || day >= previous[1]) unique.set(row[0], [row[0], day, row[2], country]);
    else previous[3] = country;
  }
  return Array.from(unique.values());
}
function updateRows(rows, id, version, today, country = null) {
  // Also repairs duplicate rows left by an interrupted older collector write.
  const next = currentRows(rows, today);
  const found = next.findIndex(row => row[0] === id);
  const firstCountry = validCountry(country) ? country : '';
  if (found >= 0) next[found] = [id, today, version, next[found][3] || firstCountry];
  else {
    if (next.length >= MAX_INSTALLATIONS) throw new Error('Capacity reached');
    next.push([id, today, version, firstCountry]);
  }
  return next;
}
function countryCounts(rows, today) {
  const countries = new Map();
  for (const row of currentRows(rows, today)) {
    if (row[1] > today || row[1] < today - 29) continue;
    const country = row[3] || 'Unknown';
    if (!countries.has(country)) countries.set(country, [country, 0, 0, 0]);
    const counts = countries.get(country);
    if (row[1] === today) counts[1]++;
    if (row[1] >= today - 6) counts[2]++;
    counts[3]++;
  }
  return Array.from(countries.values()).sort((a,b) => b[3]-a[3] || a[0].localeCompare(b[0]));
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
  const count = sheet.getLastRow() - 1;
  if (count < 1) return [];
  if (count > MAX_INSTALLATIONS) throw new Error('Capacity reached');
  return sheet.getRange(2,1,count,4).getValues().map(row =>
    [row[0], row[1] instanceof Date ? dayNumber(row[1]) : Number(row[1]), row[2], row[3] || '']);
}
function write_(sheet, before, after) {
  if (after.length === before.length && after.every((row,i) => row[0] === before[i][0])) {
    for (let i=0; i<after.length; i++) {
      if (after[i][1] !== before[i][1] || after[i][2] !== before[i][2] || after[i][3] !== before[i][3])
        sheet.getRange(i+2,1,1,4).setValues([after[i]]);
    }
  } else {
    if (sheet.getMaxRows() < after.length+1) sheet.insertRowsAfter(sheet.getMaxRows(),after.length+1-sheet.getMaxRows());
    // Write compacted records and empty tail together, without a second clear step.
    const values = after.concat(Array.from({length:Math.max(0,before.length-after.length)},()=>['','','','']));
    if (values.length) sheet.getRange(2,1,values.length,4).setValues(values);
  }
  SpreadsheetApp.flush();
}
function json_(value) {
  return ContentService.createTextOutput(JSON.stringify(value)).setMimeType(ContentService.MimeType.JSON);
}
function doGet() { return json_({service:'halcyon-activity',schema:2}); }
function doPost(event) {
  let lock;
  try {
    const report = validateReport(event && event.postData && event.postData.contents);
    // Hash the random token before storage; do not collect Google's user identity.
    const id = Utilities.computeDigest(Utilities.DigestAlgorithm.SHA_256,report.installation)
      .map(b => ('0'+((b+256)%256).toString(16)).slice(-2)).join('');
    lock = LockService.getScriptLock();
    if (!lock.tryLock(3000)) return json_({ok:false});
    const now=new Date(),today=dayNumber(now),book=book_(),sheet=book.getSheetByName('Installations');
    const before=rows_(sheet),after=updateRows(before,id,report.version,today,report.country);
    write_(sheet,before,after);
    countries_(book,after,today);
    return json_({ok:true,day:Utilities.formatDate(now,'UTC','yyyy-MM-dd')});
  } catch (_) {
    // Google may return HTTP 200 even on app errors; the client checks ok:true.
    return json_({ok:false});
  } finally { if (lock && lock.hasLock()) lock.releaseLock(); }
}
function maintenance() {
  const lock=LockService.getScriptLock();
  if (!lock.tryLock(10000)) return;
  try {
    const book=book_(),sheet=book.getSheetByName('Installations'),before=rows_(sheet),today=dayNumber(new Date());
    const after=currentRows(before,today);
    write_(sheet,before,after);
    countries_(book,after,today);
  } finally { lock.releaseLock(); }
}
// Run once as the owner after setting SPREADSHEET_ID. Rerunning is safe.
function setup() {
  const book=book_();book.setSpreadsheetTimeZone('Etc/UTC');
  const raw=book.getSheetByName('Installations');
  if (raw.getMaxRows()<MAX_INSTALLATIONS+1)
    raw.insertRowsAfter(raw.getMaxRows(),MAX_INSTALLATIONS+1-raw.getMaxRows());
  raw.getRange(1,1,1,4).setValues([['Installation hash','Last active (UTC)','Version','First reported country']]);
  raw.getRange('B2:B20001').setNumberFormat('yyyy-mm-dd');
  raw.setFrozenRows(1);
  if (!ScriptApp.getProjectTriggers().some(t=>t.getHandlerFunction()==='maintenance'))
    ScriptApp.newTrigger('maintenance').timeBased().everyDays(1).atHour(3).create();
  maintenance();
  book.getSheetByName('Summary').getRange('B9').setValue('Configured');
  return {configured:true};
}
if (typeof module !== 'undefined') module.exports = {validateReport,updateRows};

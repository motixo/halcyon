# Activity and reporting-choice collector

**Local implementation only. Live Apps Script/private Sheet migration and 0.2.1
publication remain pending maintainer review; publication is on hold.** No live
endpoint or connected Google service is needed for the tests below.

## Wire protocol and compatibility

Schema 1 (`schema`, `installation`, `version`) and schema 2 (plus `country`)
remain accepted. Neither can turn a retained opted-out row back on.
Schema 3 has two exact shapes, with no extra keys:

```json
{"schema":3,"kind":"activity","installation":"<64 lowercase hex>","revision":0,"version":"0.2.1","country":null}
{"schema":3,"kind":"status","installation":"<64 lowercase hex>","revision":1,"enabled":false,"changed_at":1792756800}
```

The first is activity; the second is the minimal choice update (no country or
version). Revisions are safe nonnegative integers; status requires revision >0.
Action time is integer UTC epoch seconds in years 1970–9999. Skew is accepted
as informational data; receipt and retention always use server time. Status
acknowledgements echo the retained `revision` and `enabled`; the client clears
only a matching latest queue. Activity acknowledges a server UTC `day`.

A higher status revision replaces current status. Equal revisions are no-ops
for matching choices; conflicting choices fail. Lower revisions cannot replace
newer ones. Activity at an existing row must have its exact revision and enabled
status; legacy activity cannot change status. A newly seen activity token can
start an enabled row, including after retention has removed the old identity.
No permanent tombstone is retained beyond the disclosed window.

The state file migrates old client state while preserving the random token.
New fields intentionally make 0.2.0's strict state reader reject it, preventing
a downgrade from silently enabling reporting. Never delete/strip those fields
to make an older client report again. Broken state fails closed.

## Schema and migration

`Installations` retains its original A:D columns unchanged during `setup()`:

1. Installation hash (SHA-256 of the random token)
2. Last active (UTC), blank for status-only installations
3. Version, blank for status-only installations
4. First reported country, blank for status-only installations
5. Reporting status: `enabled` or `opted_out`
6. Opted out at (UTC): latest off action, preserved on re-enable until aged out
7. Status received at (UTC): collector receipt of the latest distinct choice
8. Preference revision (internal ordering/idempotency)
9. Choice at (UTC): latest action time, including re-enable

Migration adds missing columns, initializes E:I for old rows and is idempotent.
It does not prune, sort, remove original rows or change spreadsheet timezone,
sharing, Script Properties, formatting or historic aggregates. An interrupted
migration can be rerun. No automatic backup duplicates identifiable data.
Normal ingestion/maintenance then deduplicates and prunes expired records.

`Countries` and a dedicated `Reporting` tab show current snapshots excluding
opted-out rows. The latter labels the retained 90-day opt-out count explicitly.
`setup()` also updates only the three exact documented legacy `COUNTIFS`
current-count formulas wherever they occur in `Summary`, adding
`Installations!E2:E20001,"enabled"`. Custom formulas and historical values remain
untouched. The maintainer must inspect any customized current-population
formulas and add that same criterion; do not reinterpret historical aggregates.
`Summary!B9` remains the existing configuration status cell.

## Maintainer deployment handoff (not performed here)

1. Keep publication on hold. Record the existing immutable Apps Script version,
   deployment ID/URL, Script Properties, trigger list, private sharing and Sheet
   timezone. Export a restricted backup of the private workbook for rollback;
   keep it outside the repository and expire it under the existing retention
   policy. Do not create public links or include tokens in logs.
2. First exercise `setup()`, reports and maintenance on a separate **private test
   Sheet**, including duplicate rows and old/new status dates. Use synthetic
   tokens only. Verify original A:D values and all unrelated workbook settings
   survive migration, repeated setup and a simulated interruption.
3. Upload `collector.gs` and the unchanged `appsscript.json` to the existing
   Apps Script project. Keep `SPREADSHEET_ID` pointing to the intended private
   workbook. Run `setup()` as owner before switching the live deployment.
   Review Summary formula coverage and Reporting/Countries snapshots. Migration
   preserves timezone; verify the intended UTC date conventions explicitly.
4. Create a new immutable Apps Script version and update the existing web-app
   deployment (owner execution, Anyone access). Keep the endpoint URL stable.
   Existing schema 1/2 clients remain supported throughout. Do **not** release
   the schema 3 client against a schema 1/2 collector: its choices would fail.
5. After approval for live verification, run the health gate:
   `python3 scripts/usage/release-check.py --check-live`. It now requires schema 3.
   A GET alone does not prove writes: use synthetic status-only, enabled activity,
   opt-out, replay, stale activity and explicit re-enable requests; check one
   hashed row, unchanged last-active on status, identical receipt after replay,
   and current counts. Remove only those synthetic rows afterward.
6. Verify the daily `maintenance` trigger, retained-row cap (20,000), retention
   and privacy of the workbook. Confirm status expiry uses receipt, not action
   time. Verify no opted-out row contributes to current Summary/Countries counts.
7. Only the parent/maintainer can approve publication after all other release
   gates. Installing this code or passing local tests does not lift the hold.

## Rollback

Before accepting schema 3 traffic, the old immutable collector version and the
restricted pre-migration backup can restore the former deployment. Prefer to
leave the additive columns; they do not require erasing A:D.

**After any opt-out has been accepted, do not roll back to the old collector or
restore an old workbook over current status:** either would discard suppression.
Keep this schema 3 collector in place while rolling back desktop code (the old
client fails closed on upgraded state), or temporarily disable the web-app
endpoint and maintenance trigger while preparing a forward fix. Retain the
current private Sheet E:I data and newer revisions. Merge any backup recovery
by installation hash and highest revision, favoring off on equal-revision
conflicts; verify in the private test Sheet before restoring service. Never
remove off state or rotate tokens to work around compatibility. During downtime
the latest client choice retries for only seven days, so count delivery may be
incomplete; local opt-out remains effective. No rollback here is executed live.

## Local verification

```sh
cargo +1.93.0 test --locked --offline --target-dir /tmp/halcyon-app-target usage:: -- --test-threads=1
cargo +1.93.0 test --locked --offline --target-dir /tmp/halcyon-app-target settings::privacy_tests:: -- --test-threads=1
node --test scripts/usage/collector.test.cjs
python3 -m unittest discover -s scripts/usage -p 'test_*.py'
```

Rust tests use loopback HTTP only and isolated state. Node tests use fake Sheets,
locks, properties and trigger services. Capture/debug network guards are retained.
The public endpoint is not authenticated: fabricated reports and copied/deleted
local state can affect estimates. The token is pseudonymous, not an account or
person count; a shared embedded secret would not authenticate installations.
See [Privacy](../../docs/PRIVACY.md) for the user-facing disclosure and retention.

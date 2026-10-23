# Daily activity collector

The client is in `src/usage.rs`. The collector is a standalone Google Apps Script
project; it writes to a private Google Sheet. See [the public disclosure](../../docs/PRIVACY.md).
The endpoint is public, but the spreadsheet and the owner's Google credentials
must remain private. The app does not need a Google login.

## Deployment

1. Use the reporting owner's Google account. API-based setup requires enabling
   **Google Apps Script API** in [user settings](https://script.google.com/home/usersettings).
2. Create a private spreadsheet with `Summary` and `Installations` tabs. The
   `Installations` headers are `Installation hash`, `Last active (UTC)`, `Version`, `First reported country`.
   The summary uses these formulas for today, seven days and 30 days respectively:

   ```text
   =COUNTIFS(Installations!B2:B20001,">="&TODAY(),Installations!B2:B20001,"<="&TODAY())
   =COUNTIFS(Installations!B2:B20001,">="&TODAY()-6,Installations!B2:B20001,"<="&TODAY())
   =COUNTIFS(Installations!B2:B20001,">="&TODAY()-29,Installations!B2:B20001,"<="&TODAY())
   ```

3. Upload `collector.gs` and `appsscript.json` into the Apps Script project.
   Set the `SPREADSHEET_ID` Script Property. This is configuration, not a secret.
4. Run `setup()` as the owner and authorize the requested spreadsheet/trigger
   access. It sets UTC, upgrades existing three-column records without changing
   identities, creates a `Countries` tab and installs one daily retention trigger. The summary's `B9` cell changes to `Configured`.
5. Deploy a **versioned web app**, executing as the owner, accessible to **Anyone**
   (including people without a Google login). If the Workspace administrator
   forbids that access, stop and resolve the restriction with the owner.
6. Put its production `/exec` URL in `assets/usage-endpoint.txt`. Do not use the
   editor's `/dev` URL. No password or owner token belongs in that file.
7. Run `python3 scripts/usage/release-check.py --check-live`, then exercise a POST
   with a fresh synthetic 64-character lowercase hex installation token and the
   release version, `schema: 2` and `country: "HR"` (or `null`). Verify `ok: true`, a server UTC date, one hashed row after two
   requests, and correct overall and country summary counts. Repeat with a different country
   and confirm the first country is retained. Verify a malformed request is rejected.
   Delete only the synthetic test row afterward; confirm the sheet remains private.
8. Check the trigger exists and run retention with synthetic old/recent dates
   in a separate test sheet. Never seed a user's real installation record.

The release workflow blocks publishing without a valid, reachable health
endpoint advertising schema 2. The older schema 1 health response deliberately
blocks this release, even if the endpoint is reachable. A health check alone does not prove that writes or retention work;
the POST/retention checks above are required before the first release or a
collector change. Update an existing deployment to keep its URL stable.

## Tests and operation

```sh
cargo test --locked usage:: -- --test-threads=1
node --test scripts/usage/collector.test.cjs
python3 -m unittest discover -s scripts/usage -p 'test_*.py'
```

No activity data is sent by these tests. Node tests exercise validation, country totals, deduplication, capacity, retention
and the request/setup handlers using an in-memory Google service fixture. Rust
client tests cover country caching and fallback.

The client looks up a country at `https://api.country.is/` without attaching an
installation ID. It caches the first success, discards the returned IP, and sends
`null` if lookup fails. It never requests city, coordinates or other enrichment.
The collector accepts schema 1 (legacy reports without country) and schema 2
(exactly `schema`, `installation`, `version`, `country`). A country is an uppercase
two-letter code or `null`; only an empty country can be filled on later reports.
`Countries` groups distinct installations active today / 7 days / 30 days by
first reported code, with `Unknown` for missing data. It refreshes after reports
and daily maintenance; the header identifies the UTC snapshot date used by all
three count windows. Expired groups are cleared. This is not download geography.

## Upgrading a schema 1 collector

Before publishing the schema 2 client, upload the new collector, run `setup()` as
the owner, make a new immutable Apps Script version, update the existing
deployment and complete the synthetic verification above. Do not install this
client against the older collector: its new reports will be rejected.

The collector holds at most 20,000 current installations and serializes writes
with a script lock. Rows inactive for 90 days are removed on ingestion and by
the daily trigger. The collector does not log bodies or expose a read API.
Check Apps Script execution failures and trigger health as part of maintenance;
the clients tolerate downtime and retry without delaying the desktop.

These are approximate counts: a public endpoint can receive fabricated reports,
and copied or deleted local state affects identity. Embedding a shared secret in
the public app would not authenticate an installation. If volume approaches
Apps Script quotas or the row cap, migrate the collector before increasing the
cap; do not let analytics outages affect the widgets.

Google references: [web app manifest](https://developers.google.com/apps-script/manifest/web-app-api-executable),
[manifest logging](https://developers.google.com/apps-script/manifest/),
[Execution API prerequisites](https://developers.google.com/apps-script/api/how-tos/execute).
The Execution API requires a shared standard Cloud project with its caller;
creating a script through the API does not by itself make remote execution available.
Running setup in the owner's editor avoids that additional API integration.

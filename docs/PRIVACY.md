# Privacy

Starting with **Halcyon 0.2.0**, the desktop process automatically sends a small
daily activity report. This applies to new installations and upgrades. Reporting
is always on, with no opt-in prompt or settings toggle. Versions 0.1.0 and 0.1.1
do not send these reports.

## Daily activity reports

The purpose is to estimate how many installations remain in use. A report has
exactly four fields:

| Field | Value |
| --- | --- |
| `schema` | Report format version, currently `2` |
| `installation` | A randomly generated 256-bit token |
| `version` | The running Halcyon version |
| `country` | Two-letter first reported country code, or `null` when unavailable |

The token is generated locally. It is not your Google identity, machine ID,
hardware identifier, username or email. It remains the same across restarts
and upgrades. Different local user accounts have separate tokens. Copying or
deleting local state can change the count, so these are estimates of active
installations, not a count of individual people or downloads.

Reports do **not** contain calendar events, music, notes, project names or paths,
weather locations, selected settings, hardware details, crash logs or screenshots.
There is no event tracking for clicks or feature use.

### Approximate country

Before the first daily report, Halcyon requests an approximate country from
[Country.is](https://country.is/) over HTTPS. Country.is receives the connection's
public IP address; Halcyon does not send it an installation ID or widget content.
The response includes an IP address, which Halcyon discards. Only the country
code is saved locally and included in the report to Google.

The first successful country result is cached across restarts and upgrades.
It is not a travel history or a live location: later VPN use, travel or moves do
not update it. VPNs, proxies and IP geolocation errors can make it inaccurate.
No GPS, precise location, weather city or account location is collected. If the
lookup fails, activity is reported with an unknown country and the lookup is
retried when another daily report is due. Country.is operates independently of
Halcyon; its network and infrastructure practices are its own.

This describes active installations' **first reported country**, not the country
of a person downloading a GitHub asset. Downloads that never run Halcyon do not
appear in activity reporting.

### When a report is sent

Reporting starts after the desktop process has been running for five minutes.
After a successful report, the client waits until the next UTC day. Failed
requests wait at least six hours before another attempt, including across
restarts. Duplicate reports update the same installation record.

Opening Settings alone does not report. Development/debug builds, tests and
isolated previews do not report. Network failures do not block the widgets or
queue reports for missed days. Installing the package alone does not start
reporting; the desktop process must be running.

### Where reports go

Reports are sent over HTTPS to a Google Apps Script web app operated by the
Halcyon maintainer. The script hashes the random installation token with SHA-256
before storing it in a **private Google Sheet**, together with the latest active
UTC date, app version and first reported country. The sheet is not publicly shared. The public endpoint
accepts reports but provides no access to stored records.

These records are pseudonymous, not guaranteed anonymous: the token permits
reports from the same installation to be linked. The collector does not store
IP addresses or log request contents. Google receives the connection's IP
address and may process infrastructure metadata under its own
[privacy policy](https://policies.google.com/privacy).

Each installation has one current record, not a history of daily events.
Records inactive for 90 days are removed from the collector's active sheet.
Google's file history, backups and infrastructure logs are subject to Google's
retention practices. The summary counts installations active today, in the last
seven days, and in the last 30 days, including a breakdown by first reported
country. Missing countries are grouped as Unknown.

### Local state

The token, first country and retry state are stored in `~/.local/state/halcyon/usage.json`
(or the corresponding XDG state directory), readable only by the local user.
They are not included in exported looks or setups. Uninstalling Halcyon stops
future reports once its desktop process has stopped; existing inactive records
expire as described above. Uninstalling normally preserves local state.

The [client](../src/usage.rs) and [collector](../scripts/usage/collector.gs) are
open source. For questions, use the project's
[issue tracker](https://github.com/profilaz3/halcyon/issues). Do not post your
installation token, credentials or private widget content in a public issue.

## Widget connections

- **Weather:** searches and selected coordinates go to Open-Meteo.
- **Google Calendar:** connecting an account allows Halcyon to fetch the selected
  calendar. Sign-in tokens stay in the system keyring. Calendar data does not go
  to the activity collector.
- **Music:** controls use local MPRIS over D-Bus. Artwork may be fetched from the
  public HTTPS URL supplied by the player.
- **Projects, notes and settings:** remain local; activity reports do not include
  their contents.

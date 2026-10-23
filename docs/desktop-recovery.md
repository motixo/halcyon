# Desktop recovery

App settings → Restore desktop previews the immutable first snapshot, its date
and scopes. Existing installations get a dated current-desktop snapshot; later
added scopes keep their own capture dates. Restore preserves unsupported records
and missing wallpaper references, shows a confirmation, and saves a separate
pre-restore point. It uses the existing interrupted-change journal and persistent
Undo. It never removes a namespace. Captured absence only removes supported keys
with durable evidence that Halcyon created them.

`desktop::recovery::{plan_restore, apply_restore}` accept recovery domains, not
setup profile domains. The private plan carries snapshot identity, strict native
byte guards, validated scoped app leaves, skipped records and missing assets.
Normal Apply retains its semantic whitespace comparison; recovery uses exact
bytes and validates both the address and the supported value schema. Undo of
Restore uses strict current-byte guards, without semantic theme regeneration.

Export contains scoped appearance/layout preferences and captured raw native
records. It excludes whole widget configuration, content, accounts, journals and
history. A checksum/version-damaged but readable manifest can export a bounded,
allowlisted **unverified evidence** envelope, which is never accepted as a restore
plan. An unreadable manifest is preserved and cannot be safely exported as scoped
evidence. Separately captured GSettings app animations are exportable, but desktop
Restore/Undo leave that preference unchanged; the confirmation states this boundary.

Restore and its Undo advance `restore-epoch.json` under config-before-desktop locks.
The monotonic generation is never rolled back, including failed/interrupted
restores. Prepared palettes and queued schedule jobs carry the generation they
observed. Palette acknowledgement (including a no-op) lives in the journaled
history record, under the same locks as writes. Interrupted transactions restore
both preferences and acknowledgement; a stale worker can do neither. The old
unjournaled acknowledgement is read only before the first restore for compatibility.

Restore is a manual choice. Current-wallpaper colors stay until the next wallpaper
input; schedules stay enabled, queued older events are superseded, and future
events follow the existing manual-override/catch-up rules. The epoch's manual date
survives history pruning and Undo. Pre-restore points under
`desktop-recovery/pre-restore` are immutable scoped evidence; the normal Undo UI
and interrupted-change recovery are the supported reversal interfaces.

# Foundation contracts

Reviewed against the local 0.3.0 tree on 2026-10-26. These are implementation requirements. [FB05](FB05.md) implements the first resolver and Clock projection, [FB06](FB06.md) shared presentation, [FB07](FB07.md) keyed Weather demand and observations, and [FB08](FB08.md) layout/input integration. [FB15](FB15.md) implements bounded Day/Night Moments. Instance migration remains FB16 work. [Examples](examples/contracts.json) record the expected outcomes.

## Applied state and compatibility

The first slice keeps `Config`, the existing profile format and the guarded store authoritative. It adds a pure projection and a few allowlisted style fields. Missing new fields preserve the old appearance. No duplicate-widget UI or second applied document is introduced here.

Current `Config` is unversioned JSON with `serde(default)`; unknown fields are ignored, and `load()` can fall back to defaults. **Adding `schema_version` cannot make an unmodified 0.3.0 client reject a newer design.** The existing profile importer has a separate, strict version boundary; do not confuse the two.

For FB16, adopt one versioned widget-domain record, `design-v1.json`, owned by the existing store. Once migration commits, it alone owns instance definitions, widget styles, visibility and placement. Existing preferences continue to own unrelated application settings and local provider configuration. Old widget fields become a frozen migration snapshot, never a second writable mirror. Runtime snapshots remain derived.

Migration requirements:

1. Stop mutable widget work; take the existing locks in their established order. Read preferences, geometry, pending recovery and history together. Unresolved recovery blocks migration.
2. Map all nine legacy kinds, including disabled ones, to stable reserved IDs such as `legacy:clock`. Preserve automatic sizing separately from explicit dimensions, display intent, data bindings and content. New duplicates receive opaque stable IDs; names and list indices are not IDs.
3. Stage the bounded record and a versioned migration receipt. Commit activation, source-domain fingerprints and owned references through the journal. Interrupted activation must finish or restore before runtime starts. No mixed legacy/new widget writers.
4. New entry points read the format header before deserializing or applying defaults. A newer/unsupported record permits explanation, export of original bytes and recovery only; it must never become an empty/default desktop. The schema version also covers snapshots and receipts used to restore it.
5. Compare frozen legacy widget-domain fingerprints on every new-client load/commit. An old client changing those fields is an external conflict, not an automatic import or permission to overwrite the new design. Unrelated legacy preference changes remain allowed.
6. Downgrade is an explicit export/restore operation. Exact export works only for representable singleton designs. Duplicates or unsupported fields require a reviewed flattening choice, with the full new record retained. Never silently discard them.

An old binary can still run against its old snapshot; the new application cannot retroactively change that binary's behavior. Do not promise universal downgrade prevention. Package lifecycle checks and process ownership can prevent supported concurrent launches, but manual old-binary execution remains an external writer. FB16 must demonstrate detection, recovery and truthful downgrade UI before activation ships.

Old history is not discarded. Migration either translates supported widget receipts into instance operations or retains a compatible legacy reader; crossing the migration boundary with Undo must be demonstrated. If that cannot be done safely, migration stays unavailable. Path and field names in the example are a contract sketch, not a committed public schema.

## Importing old setups and creating instances

Old profile versions 1–8 retain their current optional-field semantics: absent means **keep**, explicit automatic size means **remove that size override**. Applying an old Clock field targets `legacy:clock`; it does not restyle or delete a user's second clock. If that reserved instance was deliberately removed after migration, the plan explicitly proposes restoring it. Apply has one receipt; Cancel creates nothing.

Portable instance IDs are scoped to the imported design. An import plan maps them to local IDs once and retains that map through preview, Apply and Undo. Reapplying the same saved local setup uses its stable mapping; importing a separate copy creates a separate identity. Portable data declarations need local bindings. Credentials, note content and physical monitor IDs never travel with a look.

## Style resolution and editing

Resolve each property from low to high:

`recipe default → setup → moment shared → instance → moment-instance → local accessibility → supported representation`

Recheck accessibility after capability fallback. Return the effective value, originating layer and any fallback reason. No file/network discovery occurs inside resolution.

| Operation | Result |
| --- | --- |
| Change a setup accent | Changes inheriting values; leaves explicit instance accents alone |
| Edit one widget while a moment is active | Default target is that widget in the active moment; UI names the target |
| Choose “All moments” | Writes the base instance override explicitly; a higher moment override remains visible and is reported |
| Follow setup in the selected layer | Removes that layer's override; shows the next inherited source, including a lower instance override if present |
| Choose no surface | Stores explicit `none`; does not mean missing, transparent or inherit |
| Activate a moment | Changes active resolution once; does not continuously rewrite base settings |
| Manually edit after a scheduled activation | Edit persists in its named layer; no immediate re-enforcement. The next scheduled event may activate its own moment |
| Missing font or unhealthy Islands | Keep saved intent; resolve a supported fallback and show its reason |

In moments, the reset label must name the actual next source, e.g. “Use base widget style”; use “Follow setup” only when that is what will happen. FB15 uses the existing ordered rule priority and once-per-occurrence scheduler for Day/Night; solar and activity triggers remain deferred.

## First implementation boundary

FB05 adds deterministic resolution for Clock and Weather, contrasting Tidal/Paper tokens, explicit no-surface, supported font faces and origin labels. FB06 shares measured native presentation inputs. FB07 shares demand by the complete local provider key. FB08 retains current placement, input, epoch and transaction ownership. FB09 proves the complete path.

Keep singleton persistence for that proof; duplicate-provider tests may instantiate two isolated consumers without exposing persistent duplicate widgets. No recipe interpreter, new renderer or migration framework is needed for FB05.

## Local composition before instance migration

FB12 uses existing profile transport and the existing store planner. It does
not create the later design-v1 applied model. Format 10 adds optional, bounded
`composition` metadata. Records without it retain their previous minimum format;
Clock presentation remains format 9. Older readers reject the unsupported record
instead of partially applying it.

| Source | Included choices | Kept local |
| --- | --- | --- |
| Picture | Shared wallpaper, fitting and slideshow settings | Display bindings and per-display pictures |
| Style | Appearance, widget presentation, fonts, bar opacity/color/corners | Time format, date visibility, calendar/System monitor content, readability and motion preferences |
| Layout | Widget visibility/order/geometry; bar placement, dimensions, spacing and applets | Physical output assignments, window/workspace behavior |

Source profiles are frozen when chosen. Saving copies the combined values and
selected domains into one independent library record. It never applies the
preview. Source edits and deletion cannot change the saved copy. Apply and Undo
use the existing transaction; changed wallpaper assets and outside edits still
require another review.

Provenance records three original source names, kinds, optional IDs and portable
component fingerprints. It is descriptive metadata, not an asset hash, trust
signature or instruction to load another record. Portable export removes local
library IDs, personal wallpaper paths and physical output data. The current
applied Config does not claim to retain this metadata; it lives in saved setups.


## Public image export

FB13 renders the entire selected record with public samples. Apply-domain
checkboxes do not turn an image export into a partial screenshot of current
private state. The renderer cannot resolve personal/system image paths, cached
geometry or provider data. The UI and image caption identify sample content.
Selection changes cancel work; completion also compares the editor's currently
observed source/name. An already-open chooser retains its frozen image bytes.

Vertical setup plans require known disabled Islands intent and guard that key.
They never disable it implicitly. Spine is a studio addition; first-run choices
and earlier preset records remain unchanged.

# FB15 implementation boundary

Day and Night are two bounded widget presentation layers in the existing Config.
No second applied document, provider, process or timer. Config remains canonical
until FB16 migration. Neither mode changes wallpaper, app content, geometry or
native bar/window settings. This first slice covers shared widget family and
Clock face/weight/italic/surface overrides; other Moment properties stay absent.

Base settings remain intact. Shared Moment family sits above setup, ordinary
Clock overrides above shared family, and Moment Clock overrides above that.
Accessibility and capability fallbacks still win. Reset removes the selected
layer's property. The UI names Day/Night or All moments; preview selection is
local and never changes scheduling. Canvas drafts bind their editing moment.

Activation writes only the active selector through the existing store, journal
and Undo. Portable profile version 12 carries explicit definitions/selector;
missing members keep existing choices, and old captures retain their minimum
format. No schedule travels in a profile. Existing sources and content are kept.

Scheduling adds a typed Moment target to the existing automation engine. Old
rules remain version 1; records containing Moment targets require version 2.
Both versions are read explicitly. Once-per-occurrence, local DST resolution,
latest-only wake/catch-up, manual-change fencing, restore epochs, priority and
pending-rule revision checks remain owned by that engine. At coincident times,
existing ordered rule priority applies. No continuous enforcement of a style.
Sun events, four modes and activity-based switching are outside this first slice.

Acceptance: precedence/reset and invalid records; profile capture/import/Undo;
Day/Night activation and unrelated edits; manual versus queued schedule; DST,
resume, clock/timezone and restore; local preview isolation; canvas target
stability; native compact/wide keyboard controls; existing scheduler regressions.
Review and fix before proceeding to FB16.

# FB16 implementation sequence

Move the widget domain to `design-v1.json` through the existing store. Keep the
current Config as a derived runtime/editor projection; its old widget members
become a frozen migration snapshot. Unrelated preferences and provider/content
stores retain their existing owners. No installation or automatic migration of
the host's preferences is part of this work.

1. Define bounded typed instance IDs, recipe references, the versioned document,
   exact legacy-field mapping and fingerprint. Preserve all nine reserved
   instances, disabled state, ordering, absent/explicit sizes, local display
   assignments and Moment layers. Keep absent and explicit null distinct.
2. Extend the existing journal participant for the owned document and any
   simultaneous unrelated preference write. Keep config-before-desktop locking,
   conditional rollback and history. Activation must be journaled; pending
   recovery blocks migration. No independently written activation flag.
3. Read the header before projection. Newer/damaged/conflicted designs cannot
   fall through to Config defaults. Preserve last valid live observations where
   available; otherwise expose recovery without starting a default desktop.
   Compare the frozen legacy-domain fingerprint on every load and commit.
4. Route current widget edits, profile planning, local snapshots, private drafts,
   controller operations and old Undo receipts through the projection/participant.
   Retain old readers for receipts, but never maintain a writable legacy mirror.
   Bind runtime identity to the reserved instance, not a list index or a label.
5. Add explicit migration review, original-byte export and recovery. Legacy-format
   export is conditional on representability and never overwrites the source
   design. Qualify it against the retained pre-migration artifact; do not imply
   that an arbitrary older published build supports new presentation features.
6. Test interruption points, outside old-writer edits, concurrent unrelated edits,
   old/new Undo, Restore, malformed/newer schemas, import planning, controller and
   native startup/reopen. Review and fix before FB17. Duplicate creation/removal
   remains unavailable until its runtime and editor paths are qualified together.

The migration must be a real owned state transition, not a parallel shadow file.
A fresh ID is opaque and bounded. Portable IDs and local output/provider bindings
remain separate. The record's current subset must reject unsupported instance
shapes rather than silently omit an object from the desktop.

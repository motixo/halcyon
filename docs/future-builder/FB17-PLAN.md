# FB17 — personal widget recipes

Build reusable Clock and Weather compositions from native parts. Keep the existing
Clock/Weather instance and data binding; this stage does not expose duplicates or
add providers. A recipe is presentation, not executable code.

1. Bounded versioned recipe snapshots: a stable library ID, name, widget kind,
   compact/wide arrangements and a small typed part list. Clock uses face/date;
   Weather uses city/reading/conditions/forecast. Add bounded caption, divider,
   spacing and the approved settings action. Never accept commands, URLs, data
   bindings, local paths or expressions. Missing/stale Weather status is mandatory.
2. A private recipe library with atomic immutable revisions. Save/duplicate/delete
   affects the library; applying embeds the exact recipe snapshot into the existing
   canonical instance through the store. Library deletion cannot break the desktop,
   Undo or saved setups. Older designs remain readable; personal recipes require
   an explicit newer document/profile version and cannot be silently downgraded.
3. Shared native component construction for desktop, editor and public preview.
   Keep Clock face/style and Weather observations in their current owners. Choose
   compact/wide layout by available logical width; fit bounded parts into the
   existing surface and retain native text/action semantics. No network in previews.
4. A Widgets → Builder editor: choose Clock/Weather, name, add/remove/reorder parts,
   compact/wide preview and layout controls, missing-data preview, Save widget,
   Apply to Clock/Weather and Use built-in. Preview is local; apply is guarded and
   Undoable. Navigation retains the local editor draft within this Settings session.
5. Review parsing/resource bounds, IDs/revision conflicts, snapshot isolation,
   migration/downgrade, provider demand and native compact/scaled input. Fix findings,
   compile and run focused regressions, then inspect actual native captures before FB18.

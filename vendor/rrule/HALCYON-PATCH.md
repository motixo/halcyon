# Halcyon's rrule 0.14.0 patch

This is the published `rrule` 0.14.0 source, copied from the existing local
Cargo registry without changing that registry. The root manifest selects it
through `[patch.crates-io]`.

- Upstream: <https://github.com/fmeringdal/rust-rrule>
- Upstream revision recorded in the crate: `3843456f739d399cfbeff81288c36a66e8e38b83`
  (`rrule/` within that repository).
- Published crate SHA-256: `720acfb4980b9d8a6a430f6d7a11933e701ebbeba5eee39cc9d8c5f932aaff74`.
- Licenses: MIT OR Apache-2.0. The license files are from that exact upstream
  revision; the published crate omitted them.
- Copied: `src/`, `examples/`, `README.md`, `Cargo.toml`, `Cargo.toml.orig`.
  Registry bookkeeping and the dependency's standalone lockfile are omitted.

The only behavior change is in `src/iter/utils.rs::add_time_to_date`.
Resolve the generated wall time directly: a gap returns `None`, and an overlap
uses its first instant. Do not fall back to elapsed time from midnight. Both
`RRuleIter::generate` and `build_pos_list` already skip `None` before
`try_add_datetime` decrements COUNT. This fixes shifted gap occurrences and
premature COUNT exhaustion without changing iteration limits or explicit
DTSTART/RDATE parsing.

The now-unused `duration_from_midnight` helper and re-export are removed from
`src/core/datetime.rs` and `src/core/mod.rs`. The existing tests in
`src/iter/utils.rs` and `src/tests/daylight_saving.rs` now expect the gap to be
omitted and the final valid occurrence to complete COUNT. An `auditfix_`
helper regression also covers half-hour/full-day gaps and an autumn overlap.
Halcyon's parser integration regressions live in `src/calendar/subscription.rs`.
All other copied source and metadata retain their upstream contents.

Policy references: [RFC 5545 §3.3.10](https://www.rfc-editor.org/rfc/rfc5545.html#section-3.3.10)
for excluding generated nonexistent times without counting them, and
[§3.3.5](https://www.rfc-editor.org/rfc/rfc5545.html#section-3.3.5)
for choosing the first repeated wall time. Explicit gap dates keep their
separate interpretation in Halcyon's parser.

Remove this patch only after a pinned upstream release passes Halcyon's
calendar `auditfix_` regressions with the same resource limits.

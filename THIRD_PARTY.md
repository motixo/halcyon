# Third-party software

Halcyon is licensed GPL-3.0-or-later (see `LICENSE`). It is built from the
Rust crates pinned in `Cargo.lock` (534 packages for Linux x86_64, checked
2026-10-22 from the locked dependency set). Every license among them is
compatible with distributing the combined program under the GPL version 3:

| License (as declared) | Packages |
|---|---|
| MIT and/or Apache-2.0 (in their various spellings and combinations, incl. Zlib, ISC, BSD, 0BSD, Unlicense, CC0 alternatives) | the large majority |
| Unicode-3.0 | 18 (ICU and Unicode data crates) |
| GPL-3.0-only | `cosmic-client-toolkit`, `cosmic-protocols` (System76) |
| MPL-2.0 | `option-ext` |
| CDLA-Permissive-2.0 | `webpki-roots` (Mozilla's CA list) |
| Apache-2.0 OR GPL-2.0-only | `self_cell` (used under Apache-2.0) |
| MIT OR LGPL-3.0-or-later | `configparser` (used under MIT) |
| No license field in the manifest | `libcosmic`, `cosmic-config`, `cosmic-config-derive`, `cosmic-theme`, `cosmic-settings-daemon`, `iced_accessibility`, `dnd`, `mime`, `apply` (System76 git dependencies; their repositories carry MPL-2.0 or MIT license files) |

Because two dependencies are GPL-3.0-only, binaries built from this
repository are distributed under the GPL version 3.

Release packages include the full license text of every dependency
(generated from `Cargo.lock` at release time) next to the binary.

Artwork is described in `ASSETS.md`.

# Proposed FB18 implementation boundary

- A `.halcyon` JSON envelope, separate from 256 KiB legacy profiles. Version 1,
  fixed format name, portable profile, bounded credits, immutable revision digest,
  optional one shared wallpaper asset. No archive paths or executable entries.
- At most 24 MiB envelope, 16 MiB encoded PNG, 16 megapixels/8192px side and
  64 MiB decoded RGBA. Export decodes and re-encodes the explicitly included
  PNG/JPEG wallpaper to PNG, stripping source metadata. Folder slideshows and
  per-display pictures remain excluded and are named in the export review.
- No personal bindings, display IDs, notes, account data or system paths. Portable
  profile validation stays authoritative. Asset selection is a digest, never a path.
  No fonts, automatic segmentation, scripts or automatic network requests.
- Export review has an explicit Include wallpaper choice, author/license/source
  credits and a description of included/omitted data. Plain profile/preview export
  remains available. Package import is recognized by format, irrespective of suffix.
- Imported assets are staged privately under the existing store state root. A
  single writer lock, fully-written atomic publication and directory sync precede
  the library reference. Existing digest objects are verified, never overwritten.
  A failed import may leave an unreferenced immutable asset; it cannot leave a
  dangling published profile. Interrupted temporary writes are reaped under the lock.
- Keep assets referenced by applied config, drafts, saved setups, history and recovery.
  No automatic asset deletion. Bound the owned pool to 64 assets/256 MiB; explain a
  full pool rather than deleting an object that Undo may still need.
- Persist credits and origin/revision metadata inside the profile, with an explicit
  new profile version; plain export preserves credits but names omitted pictures.
  Metadata does not confer trust or activate/install Islands. Existing capability
  plans and selected-domain review remain authoritative.
- Validate independently rooted source/destination fixtures, removed source files,
  parser/image/disk bounds, interrupted staging, mismatched bytes, path/symlink
  attacks, old version refusal, native compact/wide review, Apply/Undo and recovery.

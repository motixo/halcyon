# Enable the native GPU renderer

## Context

The shipped build disables libcosmic's default features and never enables its
`wgpu` renderer. The resulting Iced renderer is tiny-skia only. On the inspected
dual-display laptop, Settings had no GPU handles or graphics-driver mappings.
During a requested scrolling/navigation sample it consumed approximately 110%
of one CPU core; an eight-second sample after interaction showed no measurable
CPU use. This establishes interaction cost, not a measured frame rate.

The existing navigation/preview changes remove repeated compatibility work and
image decoding, but do not change how the pixels are drawn.

## Decision

Enable libcosmic's existing `wgpu` feature in the application build. Use the
pinned toolkit's normal renderer and adapter selection, without forcing a GPU,
changing drivers or adding a user-facing renderer preference. This applies to
Settings, Welcome and desktop widgets because they share the application renderer.
No native backdrop effects are enabled by this change.

## Alternatives

- Keep CPU-only rendering: retains the per-pixel CPU cost at high display scales.
- Patch the toolkit or split Settings into a separate renderer binary: much larger
  scope than enabling the renderer already supported by the pinned dependency.
- Set driver/backend environment variables: would override normal selection and
  conceal whether the supported default works on the laptop.

## Consequences and verification

GPU driver/device initialization and GPU memory use become relevant. Verify the
actual selected device on the laptop, exercise both display scales, and check
idle widget CPU and rendering before treating the build as ready. Compare the
same source with and without the feature. Keep the toolkit's existing unavailable-
adapter behavior; add no custom retry or fallback mechanism. Tests that explicitly
request tiny-skia continue to cover the same native widget trees deterministically.
Record limitations separately from observed results in `docs/PERFORMANCE.md`.

# Native capability results

2026-10-26, pinned libcosmic `dc11d6a`, local 0.3.0 source. These results qualify the direction of the first slice, not a new packaged feature.

| Capability | Evidence | Initial product choice |
| --- | --- | --- |
| Font family, weight and italic | 36 real Iced paragraph cases; Noto Sans regular/bold/italic resolved to the requested faces | Expose installed faces and discrete available styles |
| Stable clock widths | `11:11`, `08:08`, `23:59` have equal widths within each tested Noto Sans and DejaVu Sans Mono face | Offer verified faces; use fixed digit cells/monospace where required, and measure punctuation and other locales |
| Missing family and mixed scripts | Missing-family requests resolved to available fonts; Croatian/Arabic sample produced no missing glyphs, using fallback faces | Preserve requested family, report fallback, measure the resolved result |
| Variable axes and OpenType feature controls | Pinned `Font` has family/weight/stretch/style; `Text` has no axes or feature tags. `to_attributes` does not forward them, despite lower-level cosmic-text support | Do not expose arbitrary axis sliders or promise `tnum` control yet |
| GPU native widget tree | Clock, Weather, Layout and Themes rendered at 820×600 and 1280×800; amdgpu engine activity observed on PCI 07:00.0 | Reuse the existing renderer and native elements |
| Software widget tree | The same four pages rendered through tiny-skia at 1280×800 | Retain the fallback; measure it separately |
| Standard vertical panel | Real stock `cosmic-panel`, private COSMIC compositor: Left anchor bits 7, Right 11; both configured to 56×800 | Spine can begin with standard native vertical panels |
| Vertical Islands | The patch has horizontal/cross-axis branches and reports observed orientation, but these probes used stock panels | Keep unqualified; no vertical-companion promise or silently rewritten intent |
| Missing/disabled/unknown Islands | Focused existing regressions exercise distinct installation/runtime states | Keep intent, compatibility and health separate |
| Depth and compositor effects | No new proof for Ridge, native blur or automatic foreground extraction | Dedicated later gate; flat supported fallback |

The font test uses `iced_graphics::text::Paragraph::with_text`, the same shaping path used by the pinned renderer. Equal digit widths in these fonts do not prove all installed fonts have tabular digits. Italic Arabic fell back to an available regular Arabic face; missing requested styles must remain visible to the user.

Source references: `iced/core/src/font.rs`, `iced/core/src/text.rs`, `iced/graphics/src/text.rs::to_attributes`, `iced/graphics/src/text/paragraph.rs` within the pinned checkout; [native panel patch](../../companion/patches/0001-dock-islands.patch); [panel model](../../src/desktop/panels.rs). Source hashes and raw results are in [baseline evidence](evidence/baseline-20261026.json).

Reproduction is described in [probes/README.md](probes/README.md). Physical hotplug, mixed scales, applet menus, screen readers and the final package remain integration/release gates.


FB13 qualifies Spine through an actual stock panel using the applied profile in
private COSMIC: Left anchor 7, 64×784 after the real Clock applet loads on an
800-high output with 8px top/bottom margins. Both final Settings sizes apply and
undo it successfully. The public PNG path uses native headless TinySkia; regular
Settings remains GPU-rendered. No vertical Islands qualification is implied.

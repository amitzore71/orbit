# UI and icon review — 19 October 2026

The shared action component now centers its vector icon beside the label, exposes selected navigation/mode state to accessibility, and keeps selected-row actions readable with high-contrast colors. Icons share a 24-unit grid, rounded caps and joins, and consistent control sizing. Calendar, power, and mini-window symbols distinguish Today, Quit, and mini widgets.

Task fields have visible labels, matching heights, and space between controls. Task-row actions are centered as a group; completed and selected tasks have matching check icons. Empty task titles disable Add. Long titles truncate before the action group, retaining their complete accessible labels. Remaining standard action buttons use the same custom component and icon family.

Report, widget, and sound controls have consistent gaps and accessible names. Scrollable pages start their content at the top. Widget headings use fixed icon slots. The timer widget is centered, and the ring track uses the same path geometry as its progress stroke at every size. High-contrast branding and ring colors follow the shared tokens.

## Verification

- `cargo test --workspace --locked --offline`: 28 passed; two opt-in hardware/resource tests remain ignored.
- `cargo clippy --workspace --all-targets --features diagnostics --locked --offline -- -D warnings`.
- `cargo fmt --all -- --check` and Python syntax checking.
- Native software-rendered captures: all six pages at 940 × 740 and 760 × 540 logical pixels, a compact timer with a recovery notice, three synthetic high-contrast screens, and all three desktop widgets. The audit uses isolated sample data with completed, selected, long, and multilingual task titles and non-empty reports.
- Rendering scales: 100%, 125%, 150%, and 200%. The script verifies physical image dimensions as well as renderer scale, and produces 19 captures per scale. This uses a renderer scale override; it does not certify mixed-monitor DPI transitions.
- Computer Use checks on an isolated database: mouse Start, Space to pause, Tasks navigation, Edit/Cancel, task entry through the native accessibility value interface, Add and form reset, Settings navigation, and scrolling to the footer. Keyboard focus outlines were visible. Physical text injection did not populate the field in this environment; direct accessibility value entry and saving did.

The restricted launch environment reported that the tray icon could not be created. Tray recovery and shell integration were not revalidated by this UI pass. Native Windows high-contrast settings were not changed; the screenshots exercise the application's high-contrast tokens.

## Reproduce

```powershell
cargo build --features diagnostics --locked --offline
python scripts/ui-audit.py
```

Each audit creates a new `artifacts/ui-audit-<timestamp>` folder, including per-scale PNGs and JSON manifests. These generated files are ignored by Git and can be removed after review. The local captures, preview executable, build cache, and old packages from this review were cleaned up; rerun the commands above to regenerate captures and a diagnostic build. ZIP and installer packages require the separate release build script.

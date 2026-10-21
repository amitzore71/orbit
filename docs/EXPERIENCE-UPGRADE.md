# Orbit experience upgrade — 21 October 2026

Orbit 0.3.1 upgrades UI, UX, animation, performance, and curated customization while retaining Orbit's offline Windows architecture and schema-4 data. It does not replace the earlier packaged release records in PERFORMANCE.md.

## What changed

- Shared typography, spacing, control, surface, and motion tokens improve readability across Focus, Tasks, Insights, Settings, and widgets. Short windows put the countdown and Start/Pause side by side instead of reducing control text sizes.
- Task filters, search, list position, report periods/custom dates, and the selected settings category survive navigation within the open workspace. Up/Down and Home/End reach virtualized task rows. Escape closes the focus picker and details, returning focus to the originating control.
- Timer controls show the configured Windows shortcut. Current-session task attribution and the task selected for the next session remain distinct. Inline validation supplements the existing backend checks and accepted-write recovery surfaces.
- Four accents (Coral, Sage, Amber, Slate Blue), two densities (Comfortable, Compact), and three motion levels (Standard, Reduced, Off) support live main-window previews. Save synchronizes widgets; Discard restores committed values. Reset appearance changes only the settings draft.
- Standard motion retains orbital progress, navigation indicators, brief page/panel transitions, task repositioning, press feedback, and a restrained session-completion effect. Reduced removes continuous movement; Off skips animation and CSS transitions. Windows/browser accessibility preferences override decorative motion and custom colors.
- Live countdown consumers subscribe independently from the workspace. One-second ticks do not commit the task list, navigation, or settings. Mutation acknowledgements and change events share revision-aware snapshot requests. Insights and Settings load on demand.
- Timer and visibility events use both window-targeted native emission and window-targeted renderer subscriptions. Tauri's default global listeners receive targeted events too, so both sides must be scoped. Hidden/minimized renderers stop decorative motion and resync on reveal.

## Settings compatibility

The JSON settings record adds `accent`, `density`, and optional `motion_level`; Rust enums validate their wire values. No SQL schema migration is required. Missing accent/density values default to Coral/Comfortable. Missing or null motion level derives from the legacy `reduced_motion` boolean; an explicit new level takes precedence and synchronizes that boolean when saved. Existing theme, durations, shortcuts, widget placement, IDs, history, backups, and restoration remain intact.

Appearance events additionally expose the system theme and system reduced-motion preference separately from saved settings, allowing an unsaved preview to be changed or discarded without overriding Windows accessibility settings. Visual previews apply to the main renderer only; native backdrop and other operational preferences retain explicit Save behavior.

## Verification

- Rust workspace: 57 tests passed; the three existing opt-in/environment-dependent tests remain ignored. Formatting passed.
- Frontend: type checking, production static export, and 6 contract/subscription tests passed. Tests verify countdown isolation, semantic timer updates, stale-revision rejection, and coalesced mutation snapshots.
- Browser: 19 journeys passed in installed Edge. Coverage includes preview/Save/Discard/Reset, late acknowledgements, rejected and accepted saves, rapid navigation, focus restoration, 5,000-task virtualization/search/keyboard access, all theme/accent/density/motion combinations, contrast for normal/hover controls, 760×540 layouts, emulated 100–200% scaling, and full visibility of controls/totals in populated and empty native widgets.
- Real WebView2/Rust: Unicode task/session binding, reports/navigation resync, all widgets, 50 widget recreation cycles, hidden-window countdown suspension while widgets remain active, main-only previews, committed widget appearance, and normal quit passed with no renderer page errors.
- Real process checks: legacy filename reuse/schema-2 migration, SQLite integrity, one-minute background expiry, injected SQLite failure/retry, abrupt termination, and durable periodic-checkpoint recovery passed without duplicate credit.
- Native power-message check: synthetic suspend/resume messages sent only to an isolated Orbit HWND close the focus interval during suspension and continue the existing running session on resume. This does not simulate Windows' hardware awake-clock behavior.

All native checks use isolated databases. Hardware sleep/wake, physical mixed-DPI monitor migration, exhaustive Windows high-contrast configurations, and clean-machine installation remain manual checks. The 0.3.1 installer and portable ZIP package this upgrade.

## Performance and remaining limits

Measured upgrade candidate before the 0.3.1 version-metadata rebuild: `target/release/orbit.exe`, SHA-256 `029F056EF1E28791C244ED83E3AEBD3564C5A756F2CFEB4A6110A33077263891`.

On the Ryzen 7 7735HS reference machine, warm startup p95 was 949ms (30 launches), interaction feedback p95 was 34.7ms (13 interactions), and sampled animation frame interval p95 was approximately 17ms. Full-process-tree running main-window mean private commit was 304.8MiB, missing the 300MiB target; all-widgets mean was 417.2MiB, within the 450MiB target. The 30-second hidden-running CPU sample measured 0.3125% of one logical core. The required ten-minute CPU soak was stopped and remains unverified. Frame sampling used renderer requestAnimationFrame intervals, not a compositor trace.

Raw results: `artifacts/upgrade/final-startup.json`, `artifacts/upgrade/final-footprint.json`, and `artifacts/upgrade/upgrade/native-performance.json`. These identify the tested candidate, not the version-metadata rebuild; earlier baseline samples are not a controlled before/after comparison. Memory and the incomplete CPU acceptance check remain release limitations.

## Review artifacts

Before screenshots are in `artifacts/upgrade/before/`; browser captures are in `artifacts/redesign/` and `artifacts/upgrade/`. Final native widget and appearance captures, the renderer CPU profile, renderer metrics, and native validation are in `artifacts/upgrade/upgrade/`. The performance section above identifies the exact executable, raw measurements, and remaining limits.

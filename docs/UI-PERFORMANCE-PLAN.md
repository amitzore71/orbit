# Orbit UI and performance revamp

Prepared 20 October 2026. Companion to [the product plan](PRODUCT-PLAN.md).

**Product promise: opens immediately, stays light, looks exceptional.**

Speed, low memory use, and visual quality are release requirements. Build the revamp on Orbit's existing Rust/Slint architecture, with the software renderer as the default. Each design decision must earn its cost in responsiveness, clarity, or visual quality.

This document records design specifications and performance targets. Orbit 0.2.0 implements the shared light/dark system, three-destination navigation, revised screens, widgets, and motion changes. §5 now separates the 0.1.1 baseline from the 0.2.0 measurements. RELEASE-NOTES.md and RELEASE-VALIDATION.md identify what shipped and what passed; PERFORMANCE.md distinguishes actual measurements from the proposed budgets below. Physical-DPI, real high-contrast, cold-cache, long-soak, and several interaction-latency gates remain unmeasured and are listed in §6. The initial audit used the source and retained UI screenshots identified in the product plan.

## 1. Design direction

Orbit should feel like a carefully made desktop instrument: precise, quiet, tactile, and immediately understandable.

The visual identity comes from four elements:

1. A distinctive, readable countdown with stable numerals.
2. Warm neutral surfaces, strong ink typography, and a restrained burnt-orange accent.
3. Carefully aligned controls, clear grouping, and comfortable spacing.
4. Immediate, subtle feedback that settles completely when interaction ends.

Keep the existing Orbit mark and native path icons. Improve their optical sizing and alignment as a family. Use system fonts and vector geometry to achieve the design without adding downloaded fonts or decorative image assets.

### Current visual audit

| Current observation | Revamp decision |
|---|---|
| The timer screen separates task context, countdown, and the main action with a large amount of vertical space. | Compose them as one focused group with a deliberate reading order. |
| Six equal navigation destinations give configuration the same prominence as daily work. | Use three main destinations: Timer, Tasks, Reports. Anchor Settings at the bottom. |
| The large countdown uses the same visual weight as other bold labels despite being the app's main object. | Increase its type scale and give supporting text a quieter hierarchy. |
| The current warm palette already has character. | Refine it rather than discard it; add a coherent charcoal counterpart. |
| The mode's accent is also used across unrelated pages. | Keep one stable product accent for navigation and actions; use mode colors only where they explain the timer's state. |
| Task rows repeat several controls and truncate long titles at compact sizes. | Give text more room; use one contextual focus action and an accessible overflow menu. |
| Settings is one long form mixing session behavior, appearance, native integration, and data actions. | Organize it into named sections with persistent Save/Discard feedback. |
| Reports combines outer scrolling with a nested task list. | Give the summary/trend a clear first position and open long detail lists in their own view. |
| Buttons already have hover, keyboard focus, and a 100 ms background transition. | Preserve and refine these states. Do not describe existing interaction feedback as missing. |
| Widgets already share the timer and have saved placement. | Apply the same typography, tokens, status wording, and control hierarchy to every surface. |

## 2. Visual system

### Color tokens

These are starting design values, to be verified in the rendered application.

| Token | Light | Dark | Purpose |
|---|---|---|---|
| Canvas | `#F5F3EE` | `#181B19` | Main window background |
| Surface | `#FCFBF8` | `#222623` | Inputs, menus, task details |
| Navigation | `#ECEAE3` | `#1D211E` | Quiet navigation region |
| Primary text | `#242923` | `#F1F2EC` | Titles, countdown, task names |
| Secondary text | `#62685E` | `#AFB6AB` | Useful supporting information |
| Separator | `#D9DDD3` | `#3B433B` | Nonessential grouping lines |
| Accent | `#A7432D` | `#F0A18A` | Primary action, active focus state |
| Text on accent | `#FFFFFF` | `#231B17` | Filled-button text |
| Selection | `#E7EBDD` | `#303A2E` | Selected task or current item |

- Set an internal contrast gate of at least 4.5:1 for normal text and 3:1 for large text and essential control indicators. Verify actual foreground/background combinations, including hover and selected states.
- Decorative separators may be subtle; an essential input outline or focus indicator needs its own stronger token.
- Errors, warnings, and success use dedicated semantic tokens plus words/icons. Never rely on color alone.
- Keep muted teal for break context if it passes the same checks. Navigation should not change color when a break begins.
- Use an opaque surface by default. Retain Mica as an opt-in enhancement with an opaque fallback.
- Add System / Light / Dark in the first UI revamp. Existing users retain the current light preference on upgrade; new installations can follow System.
- Windows high contrast overrides ordinary palettes. Theme changes update existing surfaces without restarting or constructing duplicate windows.

Planning validation: eight basic text/background pairs were checked numerically, covering primary text, secondary text, selected rows, and filled-button text across the two themes. All exceed the 4.5:1 text gate; the lowest checked pair is light secondary text on selection at 4.73:1. Rendered controls, hover states, semantic colors, and actual high contrast still require implementation-time verification.

### Typography

Use Segoe UI Variable where it renders correctly on supported machines, with Segoe UI as the fallback. Both the standard family and `SegUIVar.ttf` are present on the audit machine; this does not establish availability or behavior on every supported installation.

| Role | Starting size in logical pixels | Weight and behavior |
|---|---:|---|
| Countdown, regular window | 64–72 | Medium; stable digit positions |
| Countdown, compact window | 44–52 | Same visual identity; no clipped seconds |
| Page title | 26–28 | Semibold |
| Section title | 17–18 | Semibold |
| Task title / primary content | 14–15 | Regular or medium |
| Button label | 14 | Semibold |
| Metadata | 12–13 | Regular; adequate contrast |

- Verify digit width and baseline stability in Slint. Keep the existing Consolas option as the fallback if the chosen system face cannot produce a stable countdown.
- Avoid bundling a new font just to change personality; spacing, hierarchy, and optical alignment should do the work first.
- Use sentence case. Remove persistent decorative labels such as “ORBIT / FOCUS TIMER” if the window identity is already clear.
- Keep metadata readable rather than shrinking it to accommodate more controls.
- Truncated task titles need a keyboard-accessible full-title detail view; a hover tooltip alone is insufficient.

### Geometry and density

- Use a 4 px spacing base with preferred steps of 4, 8, 12, 16, 24, and 32 logical pixels.
- Start with 28–32 px content margins and 20–24 px in compact layouts.
- Use 8 px control corners, 10–12 px popover/detail corners, and pill shapes only when the control's purpose warrants them.
- Prefer alignment and surface changes for grouping. Reserve shadows for floating menus/popovers; avoid full-window blur, stacked translucent panels, and nested card borders.
- Give primary controls 40–44 px height; icon controls retain at least a 36 px hit target.
- Use approximately 60–68 px task rows, allowing more height for wrapped/full-title states.
- Start the navigation rail at 152–160 px. Keep visible labels; do not create an unfamiliar icon-only mode just to save width.
- Retain the 940 × 740 regular and 760 × 540 compact acceptance sizes initially. Verify physical usability at every supported DPI, including limited monitor work areas.
- At compact height, reduce nonessential spacing before shrinking controls. Start/Resume, task context, and blocked-save status must remain visible.
- Do not add a permanent dashboard, task side panel, or onboarding carousel to the timer screen.

## 3. Screen-by-screen revamp

### Timer

The visual reading order is: task → time → action → next step.

- Use a small, calm page heading and a compact Focus / Short break / Long break selector.
- Place the task picker immediately above the countdown. Its unassigned state reads “Focus without a task,” with an adjacent add/select affordance.
- Center a thin progress ring around the large countdown. Use the existing native path approach; draw progress at the existing one-second display cadence.
- Place the state label under the time: Ready, Focusing, Paused, or Saving interrupted.
- Keep “Next focus: …” separate from the current task when selections differ.
- Put Start/Resume/Pause directly below the timer in a strong, comfortable button. Keep Reset and Skip visually secondary.
- Show cycle progress and today's focus total in one restrained footer row. Calculate/update them when their data changes, not by rerunning reports every second.
- A completed session replaces the status area with “Focus complete” and the appropriate next action. Keep the layout stable and avoid foreground stealing.
- During storage failure, retain the recognizable timer layout, pause it, and show Retry saving in a persistent status region. Technical error details can expand below it.

**Visual acceptance:** at both window sizes, a user can identify the current task, remaining time, and primary action without scrolling. Empty, running, paused, completed, next-task-selected, and save-blocked states all have designed layouts.

### Tasks and Today

- Use a clear header with the task count, a compact search field, and a quiet Add task action.
- Show Today / All active / Completed / Trash as view choices. Keep the active view obvious without filling the page with oversized segmented buttons.
- Give task titles most of the row width. Put session progress beneath the title in readable secondary text.
- Represent the selected next task with a subtle background and explicit label; reserve the primary Start focus action for a session that can actually start.
- Keep the completion control visible. Place editing/deletion/reordering in a clear secondary action area or overflow menu.
- Make reordering keyboard-accessible and preserve scroll position after task actions.
- Use inline quick-add for title and an optional estimate control. Existing estimates remain supported; users can accept the existing default without extra entry.
- Place Undo feedback in a stable bottom region that does not cover rows or move the list.
- Compose empty states with a short useful sentence and one action. Use the Orbit mark or a small path icon if helpful; avoid decorative image downloads.

**Visual acceptance:** a 40-character title remains useful at compact size; complete, select, and edit are distinguishable; adding a task preserves the user's place; the 5,000-task fixture still scrolls and filters smoothly.

### Reports

- Put focus duration, completed sessions, and period selection in a compact top area.
- Show a simple native daily chart next, followed by the task breakdown.
- Prefer one primary page scroll. Open the full task/session detail list separately if a long list would otherwise require a scroll area inside another scroll area.
- Use aligned tabular numbers and restrained bars. Keep task names readable and reveal exact values through keyboard-accessible details.
- Present partial/in-progress time explicitly. Do not make an incomplete session look like lost work.
- Provide a quiet Export action when the planned export feature is implemented.
- Show pending period changes without flashing blank content or presenting old data as belonging to the new period.

**Visual acceptance:** the selected date range and headline totals are visible immediately; task totals are inspectable without relying on chart color; old results cannot appear under a newly selected period label.

### Settings

- Use a section selector for Sessions, Notifications & sound, Appearance, Shortcuts & startup, Widgets, Data, and About.
- Keep the section heading and supporting copy concise. Align labels, input widths, and section spacing.
- Use a persistent footer for Unsaved changes / Save / Discard. Status must reflect acknowledgment, not merely a button click.
- Prefer ordinary native-feeling inputs and explicit inline validation to highly custom controls.
- Put widget Advanced controls behind disclosure, with recovery actions still easy to find.
- In Data, prioritize Create backup and Restore backup; show path and technical metadata as supporting detail.
- Keep AboutSlint attribution and licensing accessible after moving About.

**Visual acceptance:** a user can see whether edits are unsaved, saving, saved, or failed; page navigation never silently discards a draft; the footer does not hide compact-screen content.

### Widgets and tray

- Ring widget: same countdown face, thin ring, clear mode, and a compact start/pause action.
- Mini timer: stable numerals, a restrained state marker, one primary control, and a reliable native drag region.
- Tasks widget: Today membership and order, readable rows, and a clear path back to the app.
- Use the same surface and semantic tokens across all widgets. Preserve positions and independent visibility.
- Keep a visible way to close each widget and a tray route to recover click-through windows.
- Preserve shared timer truth. A visual change in one surface must never create a second timer or separate selection state.

**Visual acceptance:** controls remain responsive after dragging; layout survives monitor changes; main window, widgets, tray text, and accessibility labels agree.

## 4. Interaction and motion budget

The interface responds immediately. Short transitions may finish after an action begins; actions never wait for an animation to finish.

| Interaction | Proposed treatment | Cost limit |
|---|---|---|
| Pointer hover | Small background/foreground change | 70–100 ms; affected control only |
| Press | Immediate pressed color and optional one-pixel optical offset | No delayed callback or layout reflow |
| Keyboard focus | Clear focus indicator | Immediate; no fade-in dependency |
| Selection | Background/text update | Immediate; optional transition ≤100 ms |
| Popover | Small fade if it stays cheap in the software renderer | ≤120 ms; keyboard-ready immediately |
| Navigation | Immediate content change | No full-window slide, stagger, or artificial loading delay |
| Completion | One restrained state/color change | ≤160 ms, then fully idle |
| Countdown | Stable digit and ring updates | Once per second while visible; no continuous sweep |
| Reduced motion / high contrast | Static transitions | Preserve all status information |

No looping pulse, shimmer, rotating logo, confetti, animated background, or continuously moving chart. For genuinely slow operations, show a static “Saving…” / “Creating backup…” status with truthful completion feedback.

Do not assume browser advice about GPU-friendly transforms applies to Slint's software renderer. Measure the actual affected region, raster work, CPU cost, and allocations.

## 5. Performance evidence

### 0.1.1 baseline

The baseline below comes from the recorded 0.1.1 release, not a new run:

- EXE: **10,753,024 bytes**.
- SHA-256: `0782BFC1D5AE948E7BA77693D5982B11049C7777839F9599558B8709D8917682`.
- Warm tray-launch median to native input-idle: **23.54 ms**, excluding the first of seven launches.

| Recorded scenario | Mean private commit | Mean total working set |
|---|---:|---:|
| Tray idle, fresh process | 3.48 MiB | 20.14 MiB |
| Tray running, fresh process | 3.45 MiB | 20.17 MiB |
| Main window paused | 10.09 MiB | 27.48 MiB |
| Main window running | 10.30 MiB | 27.54 MiB |
| Main window minimized, running | 10.07 MiB | 27.47 MiB |
| Main window and all widgets running | 12.41 MiB | 30.07 MiB |

Sources: [0.1.1 footprint](release-0.1.1-footprint.json), [0.1.1 startup](release-0.1.1-startup.json), and [measurement scope](PERFORMANCE.md).

### 0.2.0 measurement

Measured on the packaged 0.2.0 executable (`AB091AADEC257C60DCD0A16D6D4E98276E3851034EA2EF5979664C0AD1BB513B`, 11,670,016 bytes):

| Measurement | 0.1.1 | 0.2.0 | Change |
|---|---:|---:|---:|
| EXE size | 10,753,024 B | 11,670,016 B | +8.5% |
| Warm tray launch to native input-idle | 23.54 ms | 22.80 ms | −3.1% |
| Warm launch to painted, responsive window | not measured | 133.98 ms median / 141.27 ms p95 | new probe |
| Tray idle, fresh process: private / working set | 3.48 / 20.14 MiB | 3.43 / 20.41 MiB | −1.4% / +1.3% |
| Main window paused: private / working set | 10.09 / 27.48 MiB | 10.21 / 28.36 MiB | +1.2% / +3.2% |
| Main window running: private / working set | 10.30 / 27.54 MiB | 10.13 / 28.39 MiB | −1.7% / +3.1% |
| Main minimized, running: private / working set | 10.07 / 27.47 MiB | 10.20 / 28.37 MiB | +1.3% / +3.3% |
| Main plus all widgets: private / working set | 12.41 / 30.07 MiB | 12.25 / 30.91 MiB | −1.3% / +2.8% |

Sources: [0.2.0 footprint](release-footprint.json), [0.2.0 input-idle startup](release-startup.json), [0.2.0 painted-window startup](release-window-startup.json), and [build identity](build-info.json).

The 0.2.0 build meets the warm usable-window target (median ≤150 ms, p95 ≤300 ms), the memory ceilings, and the 12,000,000-byte package limit. The retained input-idle run is at the 0.1.1 baseline (22.80 ms versus 23.54 ms). An earlier run of the same executable measured 39.35 ms, so warm startup is sensitive to machine state and should be sampled again on the release machine. The packaged executable has 329,984 bytes (2.75%) of size headroom, so further UI additions need an explicit size check.

The footprint samples use empty isolated databases, two seconds of warm-up, and eight seconds of sampling. They do not quantify large datasets, long-term memory growth, or battery use. Some zero CPU counters are below measurement resolution; they do not prove zero work.

Private commit and resident working set are different quantities. Do not market 3.43 MiB as total RAM usage. Native input-idle is not first visible paint or first usable interaction.

## 6. Proposed performance gates

These are engineering targets, not achieved results. Establish comparable baselines first. Record the CPU model, RAM, Windows build, display scale/resolution, power mode, dataset, binary hash, and measurement scope for each run.

Use a daily-use reference machine plus a lower-powered Windows 11 x64 machine, initially targeting an SSD-equipped system with 8 GB RAM and integrated graphics. Select and name the actual devices before accepting numbers.

### Latency

| Measurement | Initial target | Definition |
|---|---:|---|
| Warm launch to first usable main window | Median ≤150 ms; p95 ≤300 ms | Window painted, core controls enabled, first action can be accepted |
| Cold launch to first usable main window | Median ≤750 ms; p95 ≤1,500 ms | Controlled reboot-based runs, recorded separately from warm launches |
| Tray activation to usable window | p95 ≤100 ms | Existing process, including after long inactivity |
| Input to visible control feedback | p95 ≤50 ms | Real input dispatch through the first observable feedback paint |
| Navigation to an already available page | p95 ≤100 ms | Excludes a separate asynchronous historical query, which remains visibly pending |
| Task search/filter response | p95 ≤100 ms with 5,000 tasks | Keystroke to updated visible results, including any debounce |
| Scrolling and brief transitions | p95 frame time ≤16.7 ms on a 60 Hz reference display | During actual interaction; investigate individual stalls above 50 ms |

The launch targets require new instrumentation; the existing 23.54 ms input-idle number cannot satisfy them. Measure startup including data loading, and report a large-dataset scenario separately. Loading a shell that cannot start a timer does not count as usable.

### 0.2.0 gate status

| Gate | 0.2.0 evidence | Status |
|---|---|---|
| Warm launch to usable window | 133.98 ms median, 141.27 ms p95, 30 trials, empty database, 100% scale | Pass (warm only) |
| Cold launch to usable window | Not measured; no reboot-based runs | Outstanding |
| Tray activation to usable window | Not measured separately | Outstanding |
| Input to visible feedback | Not measured | Outstanding |
| Navigation latency | Not measured | Outstanding |
| Task search/filter with 5,000 tasks | Model tests pass; no event-to-results measurement | Outstanding |
| Scrolling/transition frame time | Not measured | Outstanding |
| Memory ceilings | All empty-fixture private-commit and working-set ceilings pass (§5) | Pass for measured scope |
| Tray CPU including checkpoints | No 10-minute sample | Outstanding |
| Visible timer CPU | Short samples only; main-paused 0.19% in one run | Outstanding |
| Settled paused/hidden | Short sample only; not a long idle soak | Outstanding |
| Packaged EXE size | 11,670,016 of 12,000,000 bytes | Pass, 2.75% headroom |
| 500-cycle lifecycle | Not rerun for 0.2.0 | Outstanding |
| Warm input-idle | 22.80 ms median; 3.1% below 0.1.1; an earlier run measured 39.35 ms | Pass for retained run; noisy |

### Memory, CPU, and package size

Initial absolute ceilings apply to warmed, settled empty/small-fixture scenarios at the regular window size and 100% scale. Large fixtures and higher DPI must also be measured and compared with a matched baseline; these ceilings are not universal per-user promises.

| Measurement | Initial ceiling or gate |
|---|---|
| Fresh tray private commit | ≤6 MiB |
| Main window running private commit | ≤16 MiB |
| Main window plus all widgets private commit | ≤20 MiB |
| Fresh tray total working set | ≤30 MiB |
| Main window total working set | ≤40 MiB |
| Main plus all widgets total working set | ≤48 MiB |
| Tray-only running CPU | <0.1% of one logical core over a 10-minute sample including periodic checkpoints |
| Visible running timer CPU | ≤1.5% of one logical core on the reference machine; seek a reduction from the recorded baseline |
| Settled paused/hidden UI | No periodic visual redraw or decorative animation; CPU target <0.1% of one core |
| Packaged EXE | Preserve the existing ≤12,000,000-byte limit |
| Repeated open/close/navigation lifecycle | No continuing private-memory, handle, GDI, or USER resource growth after warm-up |

Treat the ceilings as failure limits, not memory allocations to spend. Also investigate a repeatable >10% regression in matched private-memory or startup measures, even if an absolute ceiling is met. A purely cosmetic change should normally fit within measurement noise; simplify it if it does not.

Measure fresh tray startup and tray-after-use separately. Showing windows can retain legitimate caches; returning to the tray does not imply the process will immediately return to its fresh-launch footprint. Measure plateau behavior and document retained memory without forcing working-set trimming.

For lifecycle stability, run at least 500 open/close cycles after warm-up and a one-hour idle/running soak. Compare batch medians and investigate continuing growth; a single before/after sample is insufficient.

“Fastest” is the product ambition. Public comparative claims require named competitors, matched tasks and machines, current versions, and published methodology. The release evidence should stand on its own: measured launch latency, response time, memory, and CPU.

## 7. Engineering tactics that preserve speed

### Rendering and assets

- Keep the software renderer as the default. Compare the existing optional GPU build only if profiling identifies a real bottleneck; judge CPU, memory, startup, compatibility, and energy together.
- Reuse the current native icon paths and ring geometry. Add only the missing glyphs needed by actual controls.
- Use opaque fills, a small number of native paths, and flat component hierarchies.
- Avoid decorative raster assets, icon font packages, embedded browser runtimes, and general animation/chart libraries.
- Update countdown strings and ring geometry only when the visible value/progress changes; share the result across visible surfaces.
- Cache derived geometry only where profiling shows a benefit, with bounded storage. Do not replace cheap computation with an unbounded cache.
- Preserve the absence of one-second visual updates on hidden/minimized timer surfaces. Keep periodic durable checkpoints separate from visual scheduling.
- Check whether paused timer properties and unchanged global tokens are being reapplied unnecessarily before introducing a broader render refactor.

### Models and data work

- Preserve stable list models and the existing ID-to-row selection updates.
- Keep rendered task delegates proportional to the visible viewport.
- Reuse filtered model data where practical. Account for duplicated task titles when adding Today, Completed, Trash, and search models.
- Make filtering responsive at 5,000 tasks. Start with the simplest measured implementation; move/coalesce expensive work if it actually exceeds the interaction budget.
- Keep reports lazy and asynchronous. Do not precompute every period at launch.
- Cache only a bounded number of useful report results and invalidate them when their inputs change, if measurements justify caching at all.
- Retain fixed-size mutation acknowledgments, bounded queues, and coalesced snapshots. Avoid rebuilding every list on each countdown tick.
- Keep backup/restore and historical queries off the UI thread. Ensure slow backup work cannot leave the app appearing frozen.
- Keep query/index changes evidence-driven; validate report totals after the new checkpoint cadence increases interval counts.

### Startup and lifecycle

- Keep first launch free of network requests and dependency downloads.
- Initialize the visible timer and required persisted state before optional reports and unopened widgets.
- Profile the initial full-task load using large fixtures; if it dominates first usability, stage task-list loading while ensuring the bound task and timer state are correct.
- Evaluate lazy widget creation against the existing implementation before proposing new lifecycle code.
- Retain deterministic disposal of native hooks, tray resources, and background operations.
- Keep executable compression opt-in; the repository already records the startup/private-memory tradeoff of compressed executables.
- Use diagnostics-only instrumentation. The normal package should not acquire a profiling or image-capture dependency.

## 8. Implementation slices and estimates

The UI work is integrated with the product roadmap. These estimates represent **8–12 additional engineering days**, beyond functional tasks already counted in that plan; do not add both the full UI estimates and duplicate functional estimates for the same controls.

| Slice | Deliverable | Incremental estimate | Exit gate | 0.2.0 status |
|---|---|---:|---|---|
| V1 — Design and measurement baseline | Tokens, typography, light/dark state samples, benchmark definitions, regular/compact layouts | 2–3 days | Contrast/fit checked; baseline scope and hardware recorded | Delivered; contrast pairs checked mathematically, physical rendering outstanding |
| V2 — Shell and Timer | Three-destination navigation, refined timer composition, common actions, theme integration | 3–4 days | Every timer state fits; launch/input/memory comparison passes | Delivered; launch/memory measured, input latency outstanding |
| V3 — Tasks, Settings, Reports, widgets | Apply the shared system to existing and newly planned controls; preserve functionality | 2–3 days | Consistent states, complete keyboard journeys, large-list checks | Delivered; rendered states and model checks pass, physical keyboard/DPI journeys outstanding |
| V4 — Visual and performance release pass | Real-DPI/accessibility checks, long samples, lifecycle soak, release comparison | 1–2 days | Both visual quality and measured budgets pass | Partially complete; warm launch, memory, and size pass, long samples and lifecycle soak outstanding |

These are rough increments for a familiar engineer, with design and QA support. Investigating a newly discovered native issue or building robust first-paint instrumentation can require additional time.

The combined first product-and-UI release is approximately **33–50 engineering days before contingency**, or **8–12 working weeks including 20% contingency** under the product plan's staffing assumptions.

Ship a focused shell/Timer visual slice earlier if useful, after its own regression checks. Keep new Today/Trash controls tied to their functional implementations; no decorative controls with unimplemented actions.

## 9. Verification and evidence

1. Save matched before/after screenshots of regular and compact windows, both themes, real high contrast, and each error/empty/pending state.
2. Extend the existing UI audit rather than starting an unrelated browser test stack. Include 100%, 125%, 150%, and 200% scaling plus real monitor movement.
3. Extend `scripts/measure-startup.py` with a separately identified usable-window measurement. Corroborate an internal first-render marker with external UI observation; a callback by itself does not prove a visible frame.
4. Extend `scripts/benchmark-ui.py` beyond model-only selection timings to include event-to-feedback, search, navigation, and scrolling scenarios. Keep the old microbenchmark labeled accurately.
5. Use `scripts/Measure-Footprint.ps1` for comparable private commit, working set, CPU, and cycle counters. Add long sampling and post-use tray scenarios.
6. Measure at least 30 warm launches and 100 repetitions of each common interaction; report median, p95, sample count, and outliers. For cold launches, collect independent reboot-based runs and disclose the sample size; fewer than 20 runs supports only an exploratory tail estimate.
7. Pair before/after runs on the same machine and fixture; alternate order where practical. Exclude only documented harness failures, not slow valid samples.
8. Use 0-task, small everyday, 5,000-task, and long-history fixtures. Include checkpoints, notifications, window dragging, theme changes, and storage recovery in realistic combinations.
9. Run the existing functional, failure-injection, and packaging checks. A beautiful timer with changed time accounting fails the release.
10. Publish a compact comparison in `docs/PERFORMANCE.md`, identifying the exact packaged binary. Report regressions and scope limitations alongside improvements.

Review each screen at normal scale before zooming in. The final visual pass should check numeral stability, text baselines, icon optical alignment, truncation, focus indicators, disabled-state legibility, popover placement, and scrollbar behavior.

## Definition of done

- The countdown, current task, and primary action form a clear visual group.
- Light and dark appearances are coherent across the main window and all widgets.
- Every changed control has designed default, hover, pressed, focus, disabled, pending, and error states where applicable.
- Normal work requires no extra configuration or onboarding to access.
- The interface settles completely after interaction; ongoing CPU work has a functional reason.
- Matched measurements meet the agreed launch, interaction, memory, CPU, and size gates.
- Existing history, task binding, sleep exclusion, recovery, native drag, and accessibility behavior remain correct.

The first visual slice should establish the shared tokens and refined Timer composition while retaining the existing renderer, icons, and persistence behavior. That provides an early, measurable proof that Orbit can look substantially better while staying light.

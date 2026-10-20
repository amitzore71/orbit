# Orbit product plan

Prepared 20 October 2026 against Orbit 0.1.1, commit `eb6e8dd`. Orbit 0.2.0 delivers the P1 scope; implementation and verification status are recorded below and in RELEASE-VALIDATION.md. Estimates, priorities, and the audit table remain the pre-implementation record.

**Recommendation:** make Orbit the fastest way to choose today's work, focus on it, and trust the resulting history. Prioritize the daily task-to-timer workflow and data recovery before expanding analytics or customization.

**Product identity:** opens immediately, stays light, looks exceptional. The UI revamp and measurable speed/memory gates are specified in the companion [UI and performance plan](UI-PERFORMANCE-PLAN.md). Visual quality and efficiency are release requirements, and System / Light / Dark shipped with the 0.2 revamp.

This document is both the pre-implementation product direction (audited against 0.1.1) and the acceptance basis for 0.2.0. RELEASE-NOTES.md describes what shipped; RELEASE-VALIDATION.md records the automated and packaged checks that passed. Phase 0 usability interviews were not run before 0.2, so the user-success targets later in this document remain proposed and unmeasured. Cold-boot testing and physical hardware scenarios also require separate evidence.

Visual evidence reviewed: the 0.1.1 Timer, compact Tasks, Reports, Settings, and Widgets fixtures from `artifacts/ui-audit-1792424875024435000/scale-1/screenshots/`, plus the 0.2 audit captures under `artifacts/ui-audit-*/`. These are rendered fixtures, not evidence of behavior under real mouse, keyboard, monitor, or theme changes.

## Product direction and scope

Assume the primary user is an individual working or studying on Windows who wants a lightweight, offline focus tool. Validate that assumption during the first phase.

The main journey should be:

1. Capture a task or choose one already planned for today.
2. Start a focus session from that context.
3. See which task is being tracked and what comes next.
4. Take the next break with a clear, optional action.
5. Review today's effort and choose the next task.

Preserve the existing strengths: no account, no telemetry, offline storage, native Windows integration, optional widgets, conservative automatic-start defaults, and low resource use. Keep focusing without a task as a first-class path.

The release delivered three substantial additions: a real Today list, backup/restore, and task recovery. Other shipped work improved existing behavior or reduced interface complexity.

## Implementation status (0.2.0)

| Scope | Status | Evidence or remaining work |
|---|---|---|
| R1–R6: navigation and task-list simplification | Delivered | Three destinations; Settings groups Widgets, Data, and About; Completed and Trash views; compact task actions; Advanced widget disclosure; Today-backed task widget. |
| A1: Today list and task selection on Timer | Delivered | Schema 4 adds `planned_for` and `today_order`; Today, Carry forward, search, ordering, Timer picker, quick-add, and Start focus / Use next behaviors shipped. See RELEASE-NOTES.md. |
| A2: backup, restore, and Data settings | Delivered with a design change | Manual backups, verified snapshots, latest-three upgrade backups, pre-restore recovery copy, and in-place restore. Restart is not required because replacement uses SQLite's backup transaction; the plan's earlier “Restore and restart” wording is superseded. |
| A3: Undo deletion and Trash | Delivered | Timed Undo, Trash restore, and retained history; restored tasks are not automatically reselected. |
| I1–I4: state clarity, checkpoints, recovery, settings | Delivered | Current/next labels, one-minute coalesced checkpoints, typed save status with Retry saving, and preserved settings drafts. |
| I5: inspectable reports | Delivered ahead of schedule (was P2) | Custom ranges up to 367 days, session detail, and formula-safe UTF-8 CSV export. |
| I6: widget and accessibility consistency | Delivered for changed surfaces | Shared tokens and wording, theme application, keyboard journeys, and widget state alignment. Real high contrast, reduced motion, monitor movement, and 125–200% physical scaling still require manual verification. |
| V1–V4: UI and performance revamp | V1–V3 delivered; V4 partly verified | Shared light/dark system and all screens shipped. Warm launch, memory, and size measurements pass; physical-DPI, long lifecycle soak, cold launch, and some interaction latencies remain outstanding in the companion plan. |
| Phase 0: five-user validation | Not executed | No interviews or recordings were collected. All usability targets remain proposed. |

## What the audit establishes

| Observed behavior (0.1.1) | Product implication | Evidence at `eb6e8dd` |
|---|---|---|
| Timer, Tasks, Reports, Widgets, Settings, and About occupy six equal navigation positions. | Daily actions compete with infrequent configuration destinations. | `crates/app/ui/app.slint:169` |
| “Today's tasks” receives all unfinished tasks; tasks have no planning date. | The label promises a daily plan that does not exist. | `crates/app/src/task_models.rs:26`, `crates/storage/src/lib.rs:27`, `crates/app/ui/app.slint:456` |
| The task-row “Focus” button selects a task without starting the timer or opening Timer. | The action's result is weaker than its label and play icon suggest. | `crates/app/ui/app.slint:262`, `crates/app/src/main.rs:730` |
| Tasks are sorted by completion status and ID. There is no search, planning order, or completed-only view. | A growing task list becomes harder to use, despite virtualized rendering. | `crates/storage/src/lib.rs:436`, `crates/app/ui/app.slint:230` |
| Deletion sets a flag and preserves history, but the UI exposes no restore action. | An accidental deletion is recoverable in the data model but not by the user. | `crates/storage/src/lib.rs:554`, `crates/app/src/main.rs:730` |
| Selecting another task preserves the active session's original task. Timer displays one task label. | Correct accounting needs clearer “current” and “next” presentation. | `crates/focus-core/src/lib.rs:230`, `crates/focus-core/src/lib.rs:372`, `crates/app/src/main.rs:463` |
| Navigating to Settings calls `load_settings`; there is no draft/dirty state. | Returning after navigation can overwrite unsaved edits in the form. | `crates/app/src/main.rs:242`, `crates/app/src/main.rs:840` |
| A dismissible shared message explains storage failures and asks the user to retry a timer control. | Recovery is hard to discover, and the visible warning can disappear while the problem persists. | `crates/app/ui/app.slint:188`, `crates/app/src/main.rs:366`, `crates/app/src/main.rs:582` |
| Persistence occurs on transitions and explicit checkpoints; ordinary display ticks do not save elapsed time. | A long uninterrupted session can lose substantial recent progress after a crash. | `crates/app/src/main.rs:366`, `crates/app/src/main.rs:539` |
| Backup instructions require closing Orbit and copying its data directory. | Data ownership is good, but protecting and restoring it requires filesystem knowledge. | `README.md`, `crates/storage/src/lib.rs:362` |
| Reports already contain calendar totals, task breakdowns, partial time, and deleted/unassigned history. | Improve inspection and export instead of rebuilding analytics. | `crates/app/ui/app.slint:274`, `crates/storage/src/lib.rs:634` |
| Main window and widgets explicitly use the light palette. | A system/dark appearance option is a later enhancement, not an existing feature. | `crates/app/ui/app.slint:101`, `:426`, `:448`, `:477` |

These are 0.1.1 implementation observations; line references apply only at `eb6e8dd` and have moved in the 0.2 source. The priority judgments below are hypotheses about user value; no usage frequency or customer demand is claimed.

## 1. Remove or simplify

Remove friction and unnecessary prominence first. There is insufficient user evidence to delete a major capability such as either timer widget.

| ID | Decision | Reason | Completion criteria |
|---|---|---|---|
| R1 | Remove About from primary navigation; place it inside Settings. | It is useful information but not a daily destination. | Version, application license, packaged notices, and the existing AboutSlint attribution remain easy to reach. |
| R2 | Move widget configuration into Settings → Widgets; retain a direct tray entry. | Widgets should help users focus without requiring a permanent top-level configuration page. | Existing visibility, positions, layering, and recovery controls survive the move. |
| R3 | Hide completed tasks from the default active list; provide a Completed filter. | Finished work should not accumulate in the working list. | Counts are visible, completed tasks can be reopened, and nothing is deleted automatically. |
| R4 | Move Delete into a task overflow menu. Retain a clear edit/detail action. | The compact screenshot shows substantial row width devoted to repeated controls. | Full titles are accessible in task details; overflow works with keyboard and screen reader; deletion has Undo. |
| R5 | Put click-through and advanced window layering behind an Advanced disclosure. | These are useful but easy to misconfigure. | “Unlock all widgets” and “Reset positions” remain immediately accessible through the tray. |
| R6 | Rename the existing task widget “Active tasks” until A1 ships. | Its current data is an active list, not a daily plan. | The “Today's tasks” label returns only when the widget actually uses Today membership. |

Keep all three widgets in the next release. Ring and mini timer overlap functionally but serve different space and visibility preferences. Consider presenting them as two display styles only after observing real use; do not merge their stored preference slots during this work.

Do not add accounts, cloud sync, AI coaching, social leaderboards, streak pressure, ambient media, a full calendar, or application blocking in this roadmap. These would create new product obligations without resolving the observed workflow gaps. Reconsider only with evidence of a recurring user need.

## 2. Add features

### A1 — A real Today list and task selection on Timer

**Priority:** P1, required for the daily-workflow release.

**Problem:** Orbit stores a backlog but offers no way to distinguish today's intended work. Starting task-based focus requires visiting a separate page and interpreting the selection behavior.

**Proposed behavior**

- Keep Timer as the launch page. Add a searchable task picker and quick-add entry next to its task context.
- Give Tasks four views: Today, All active, Completed, and Trash. Today is the default after the user has planned work; empty states explain how to add or choose tasks.
- Add “Plan for today” and “Remove from today” actions. Removing a task from Today leaves it active in the backlog.
- Support a manual order within Today. Provide Move up/down buttons as well as optional drag interaction.
- At the start of a new day, offer unfinished items from earlier days under “Carry forward.” Do not silently assign all overdue work to today.
- Do not impose a hard daily task limit. Encourage a short list through copy and layout.
- Show today's active focus time and completed focus-session count using the existing summary query.
- Make the task widget use the same Today list and order. Its empty state should say “No tasks planned for today” and offer a route to planning.
- Completing a task shows pending feedback, then removes it from Today when storage confirms the mutation.

**Task-to-timer contract**

- With a fresh, unstarted focus session, “Start focus” selects the task, starts the session, and opens Timer.
- During a running or paused started session, the task-row action is “Use next.” It must never silently replace the bound task.
- During a break, “Use next” selects the next focus task without ending the break.
- Timer's picker changes selection; the primary Start button explicitly starts a session.
- A storage failure must not leave a successful-looking start action. Apply select/start as one coordinated transition with rollback on rejected submission.
- If the selected next task is completed or deleted, clear that next selection. Preserve an already bound session and its attribution.
- Do not automatically advance through Today tasks. Completing a focus session is not equivalent to completing the task.

**Data and implementation**

- Add a nullable `planned_for` local calendar date and a stable `today_order` value to tasks. A task has at most one current planning date.
- Existing tasks migrate with no planning date. Preserve IDs, completion counts, titles, and selected/bound task references.
- Planning dates represent user intent and do not shift when Windows changes time zone. “Today” follows the current local calendar.
- Keep historical daily-plan analytics out of scope; this first schema stores current planning intent only.
- Extend the existing stable task models with filtered ID mappings. Preserve virtualization and efficient selection updates.
- Start search with an in-memory, case-insensitive title filter over loaded tasks. Handle Unicode without corrupting titles; defer a full-text search dependency unless measurements justify it.

**Acceptance criteria**

1. An existing task can start a new focus session from Tasks with one explicit action.
2. A new task can be created and started from Timer without visiting another page.
3. Planning, order, and selection persist across restart.
4. Main UI and task widget show identical Today membership and ordering.
5. Midnight, sleep across midnight, and time-zone changes do not silently discard unfinished tasks or rebind a session.
6. Search/filter/reordering remain usable with 5,000 tasks and preserve keyboard focus.

### A2 — Backup, restore, and a useful Data settings page

**Priority:** P1; the snapshot mechanism is a dependency for the new task schema.

**Problem:** Users currently need to find and copy the correct directory while Orbit is closed.

**Proposed behavior**

- Settings → Data offers “Create backup,” “Restore backup,” and “Open data folder.”
- A backup includes tasks, session history, settings, and a safely paused timer checkpoint.
- Show the actual data location, most recent successful backup, and whether that backup was user-selected or made before an upgrade.
- Create a verified backup before the next schema migration. Retain a small, documented set of upgrade backups, initially the latest three; never silently prune user-exported backups.
- For manual backups, let the user choose the destination. A copy on the same disk protects against some app mistakes, not disk failure; explain this briefly at the point of choosing a location.
- Restoring shows the backup date, version/schema compatibility, and replacement scope. Require an explicit “Restore” confirmation because current local data will be replaced. The 0.2 implementation refreshes in place after replacement rather than forcing a restart.
- Make a pre-restore backup. Do not merge databases in the first version.

**Engineering boundaries**

- Operate on the resolved active database path, including custom `--data-dir` locations and legacy databases reused in place.
- Pause and acknowledge the current checkpoint before taking a snapshot; keep the timer paused afterward and explain that state.
- Create a consistent snapshot through a supported SQLite backup mechanism owned by the storage layer. Do not copy an open WAL database file as the backup implementation.
- Perform potentially slow backup work off the UI thread, with bounded work and clear completion/failure states.
- Validate a restore candidate before replacing anything: file readability, database integrity, expected tables, supported schema, and loadable settings/checkpoint data.
- Stage restoration, quiesce the worker, close live connections, and handle WAL/SHM sidecars correctly. Replace the live database through SQLite's backup transaction so an interrupted copy rolls back instead of leaving a partial file. The shipped 0.2 path validates a staged candidate, writes a pre-restore recovery copy, then replaces the live database transactionally.
- Reject a backup from an unsupported newer schema without changing the active database.
- Treat native startup and notification registration as machine-specific. Restoring settings must not silently register another executable or unexpectedly enable startup.

**Acceptance criteria**

1. A user can back up, restore into an isolated directory, and recover identical task/history totals.
2. Cancellation, disk-full, permission errors, invalid files, and interruption during restore preserve a recoverable original.
3. Restore starts paused and credits no time spent outside Orbit.
4. Existing custom and legacy data paths pass the same round-trip checks.
5. A failed upgrade backup leaves the original schema untouched and offers an actionable retry.

### A3 — Undo deletion and recoverable Trash

**Priority:** P1, paired with task-list simplification.

**Problem:** Soft deletion already retains task history, but users cannot reverse it through Orbit.

**Proposed behavior**

- Show “Task deleted — Undo” after successful deletion, initially for ten seconds.
- Keep recovery available afterward through the Trash view; the time limit must not be the only accessible recovery path.
- Allow Restore from Trash. Avoid a permanent-delete action in the first version.
- Restoring preserves the same task ID, title, estimate, completion state, and session history.
- Restore to All active or Completed as appropriate. Clear obsolete planning membership; the user can deliberately plan the task again.
- Never automatically reselect or start a restored task.
- Allow completed tasks to be reopened from Completed without losing history.

**Implementation and acceptance**

- Reuse the existing `deleted` flag and add a storage restore operation. Existing deleted rows become recoverable without requiring a deletion timestamp.
- Route delete/restore through the bounded mutation queue; keep UI pending state distinct from durable success.
- Test delete → undo while a timer is active, deletion during a pending edit, repeated undo activation, and storage retry.
- A bound task that has been deleted should display its retained title with a deleted indicator, rather than misleadingly appearing to be an unassigned session.

## 3. Improve existing features

### I1 — Make timer state and transitions understandable

**Priority:** P0 for misleading state and accidental transitions; P1 for completion polish.

- Show “Focusing on” for the bound task and “Next focus” when the pending selection differs. Show “Focus without a task” for deliberately unassigned work.
- When paused, use “Resume session” instead of the generic “Start session.”
- Replace the cumulative “SESSION N” display with meaningful cycle progress, such as “2 of 4 focus sessions completed before a long break.”
- Distinguish readiness, running, paused, recovering, and storage-blocked states in the UI.
- Resetting or changing mode after any focus time has accrued should explain that elapsed time remains in reports but the session will not count as completed. Offer a lightweight confirmation for that active-session transition.
- Use explicit Skip wording appropriate to the current mode. Review the global skip shortcut: leave existing users' configuration intact, but default it off for new users to reduce accidental transitions.
- At completion, show a quiet summary and the next action: start break or start focus. Do not steal foreground focus.
- Respect existing automatic-start choices. If the next session already started automatically, display its actual state instead of presenting a stale Start action.
- When notifications are disabled or unavailable, the next action remains visible in the main UI and tray workflow.

**Done when:** UI, tray, hotkeys, and widgets agree on the same timer state; mode changes cannot silently discard a started session's meaning; completed-session counts remain exact under delayed callbacks and repeated actions.

### I2 — Reduce crash-related loss of running progress

**Priority:** P0, a targeted improvement to persistence.

The current implementation deliberately avoids inventing unsaved time after a crash. Keep that rule, but save progress more often while running.

- Schedule a background checkpoint every 60 seconds of awake running time, independent of whether a timer surface is visible.
- Reuse the deterministic core's existing `Command::Checkpoint`. Persist the closed focus interval and the new timer checkpoint atomically.
- Allow at most one periodic checkpoint request in flight; coalesce work instead of building a backlog.
- Stop periodic scheduling while paused, suspended, quitting, or storage-blocked. Preserve deadline-based expiry and avoid introducing per-second database writes.
- Track the last acknowledged saved progress so recovery copy can accurately describe what was restored.
- Do not credit time after the last durable checkpoint. Remain paused after recovery.

**Target:** under healthy storage and a responsive event loop, normally limit unsaved running progress to approximately one minute plus save latency. This is not a guarantee during storage failure, a stalled event loop, or abrupt hardware failure.

**Done when:** abrupt-termination fixtures recover the last acknowledged interval without duplication; sleep adds no focus time; session completion still happens exactly once; checkpoint-related database growth and idle/tray CPU are measured on the actual release candidate.

### I3 — Turn storage recovery into a clear user flow

**Priority:** P0.

- Replace the shared message string as the sole representation of failure with typed status: saving, saved, retrying, and blocked.
- Show a persistent “Could not save — timer paused” indicator while blocked. Dismissing technical details must not hide the blocked state.
- Add an explicit “Retry saving” button. Timer controls should describe why they are unavailable.
- On successful retry, show “Changes saved” and a separate Resume action. Never start tracking automatically.
- Preserve accepted queued edits and explain that they are pending. If a mutation was rejected before enqueueing, preserve the user's input so it can be submitted again.
- Keep routine saved confirmations near their controls; reserve the global banner for conditions affecting the whole app.

**Done when:** failure injection shows no lost accepted mutations, duplicate tasks, unbounded retries, or success state before acknowledgment. Recovery is possible without guessing which timer control to press.

### I4 — Make settings predictable and easier to scan

**Priority:** P1.

- Group Settings into Sessions, Notifications & sound, Appearance, Shortcuts & startup, Widgets, Data, and About.
- Preserve a settings draft while navigating within Orbit. Show a dirty indicator and Save/Discard actions; warn on actual app exit if a draft remains.
- Keep the existing explicit Save model and acknowledgment-before-application behavior.
- Validate duration and shortcut errors beside the relevant field. Preserve the draft when validation fails.
- Add shortcut recording with an accessible manual-entry fallback and a clear Disable control.
- Distinguish a saved shortcut preference from successful Windows registration; show conflict warnings beside the affected shortcut.
- Replace the ambiguous Notifications button with “Set up and test notifications.” State what can be confirmed locally without claiming Windows displayed a toast when that cannot be verified.
- Keep startup opt-in. Surface native registration failures without claiming the requested state took effect.

**Done when:** edits survive page changes, Save and Discard have clear effects, failed saves retain input, and startup/shortcut failures leave accurate visible status.

### I5 — Make reports inspectable and portable

**Priority:** P2, after the daily workflow is stable.

- Preserve existing day/week/month totals and task attribution.
- Rename the reset-period action contextually: Today, This week, or This month.
- Place the daily trend and summary before the potentially long task breakdown.
- Add task detail with a readable full title and a list of recorded sessions/interval-derived focus totals for the selected period.
- Separate completed sessions from partial/in-progress work. Existing stored rows do not distinguish every historical reason a session ended; do not invent “reset” or “skipped” labels for old data.
- Add a custom date range and CSV export of the selected report. A report export is not a restorable backup.
- Include period boundaries, time-zone context, stable task IDs, active duration, and completion counts. State that historical day assignment uses the current Windows time zone.
- Use correct CSV escaping and spreadsheet-safe treatment of task titles that begin with formula characters.
- Keep queries asynchronous and generation-tagged so stale results cannot replace a newer selection.

**Done when:** exported totals reconcile with the displayed report, including deleted tasks, unassigned time, midnight boundaries, DST, and partial sessions. Do not build streaks or productivity scores as substitutes for useful detail.

### I6 — Improve widget consistency and accessibility

**Priority:** P1 for changed surfaces; P2 for additional appearance work.

- Reuse one presentation model for timer wording and state across main window, ring, mini timer, and tray.
- Keep widget preferences compatible. Closing a widget must not stop the shared timer.
- Add task context where space permits; the mini timer can expose the full title through an accessible description/detail affordance.
- Preserve the native drag handling already fixed in the repository. Verify controls still work after dragging.
- Verify complete keyboard access, logical tab order, visible focus, screen-reader names, and long/multilingual titles on every changed surface.
- Verify actual high contrast, reduced motion, compact windows, and 100%, 125%, 150%, and 200% scaling.
- Add System / Light / Dark appearance as part of the first UI revamp. Apply it to custom tokens and standard controls on all windows; high contrast takes precedence. Follow the companion plan's palette, migration defaults, and performance gates.

**Done when:** a user can plan, start, pause, recover a task, and retry saving with the keyboard; widget state matches the main timer; actual monitor and Windows accessibility changes pass the manual scenarios.

## Priorities and delivery sequence

P0 means user trust or state clarity; P1 means required daily-workflow capability; P2 means a later enhancement. These priorities are product judgments, not incident severity claims.

| Phase | Scope | Dependencies and exit condition | Effort estimate |
|---|---|---|---|
| 0 — Validate the workflow | Observe current task selection, completion, settings, and recovery with five target users; capture baseline timings and misunderstandings. Record current release performance. | Confirm that daily planning is useful and identify any frequently used navigation/widget behavior before moving it. | 2–3 engineering days |
| 1 — Trust and foundations | I1 state clarity, I2 bounded progress saving, I3 recovery, A2 backup/restore, I4 draft persistence, R6 honest widget naming. | Backup and restore failure paths work before introducing task-schema changes; core accounting tests stay green. | 12–18 engineering days |
| 2 — Daily workflow | A1 Today/picker, A3 Trash/Undo, R1–R5 navigation/list simplification, remaining I4 organization. | Task migration is reversible through the verified backup; main UI and widgets use the same models. | 8–12 engineering days |
| 3 — Release hardening | I6 checks on changed surfaces, migration fixtures, installer/portable checks, fault injection, performance and usability retest. | Meet the release gates below on the packaged binary. Publish migration and recovery instructions. | 3–5 engineering days |
| UI track across Phases 0–3 | Shared visual system, System/Light/Dark, refined Timer, task/settings/report/widget styling, measured launch and interaction feedback. | Follow V1–V4 in the companion plan; do not duplicate estimates for functional controls already counted above. | 8–12 additional engineering days |
| Later — Review and personalization | I5 report functionality/export, optional named duration presets if users repeatedly change lengths. | Decide scope using feedback from the daily-workflow release. | Re-estimate; initially 6–10 engineering days |

The functional plan is approximately 25–38 engineering days before contingency. The requested UI/performance revamp adds an estimated 8–12 days, bringing the combined first release to 33–50 engineering days. With 20% contingency, plan roughly 8–12 working weeks for one engineer familiar with Rust/Slint/Windows, with part-time design and QA support. This is an estimate, not a delivery commitment; restore safety, performance instrumentation, and native desktop testing are the largest uncertainties.

If capacity is constrained, ship Phase 1 as a small trust-focused release, then Today and task recovery as the next release. Do not cut migration safety to fit more visible features into one release.

Suggested implementation order within the phases:

1. Establish visual tokens and comparable performance baselines; introduce typed save/recovery state and settings drafts.
2. Add acknowledged periodic checkpoints and clear current/next timer presentation.
3. Implement and validate snapshot/restore, including interruption recovery.
4. Add the task-schema migration and worker operations for planning/order/restore.
5. Add filtered task models, Today, and the Timer picker.
6. Add Undo/Trash and task start/next behavior.
7. Apply the shared visual system, move navigation/settings surfaces, and connect widgets to Today.
8. Run packaged-binary validation, verify the UI/performance gates, and repeat the user tasks.

## Engineering plan and compatibility

### Keep responsibilities clear

| Area | Planned responsibility |
|---|---|
| `crates/focus-core/src/lib.rs` | Timer transitions, immutable binding for started sessions, checkpoint semantics, explicit select-and-start behavior if implemented as a core command. No calendar UI or Windows code. |
| `crates/storage/src/lib.rs` | Task migration, ordered/filterable queries, restore mutation, durable acknowledgment, and report queries. |
| `crates/storage/src/snapshots.rs` | SQLite-native snapshots, integrity checks, upgrade/pre-restore backups, and transactional replacement. |
| `crates/storage/src/export.rs` | UTF-8 CSV report export with spreadsheet-safe titles. |
| `crates/app/src/task_models.rs` | Stable Today/active/completed/trash models, search, ID mapping, and selection updates. |
| `crates/app/src/product.rs` | Today/task presentation, file-dialog results, backup/restore application, and report detail/export coordination. |
| `crates/app/src/main.rs` | Orchestration, checkpoint scheduling, typed save state, settings drafts, and consistent presentation to windows. |
| `crates/app/ui/app.slint` | Navigation, task picker, filters, recovery actions, settings groups, and accessibility. Split into page/component files as those surfaces are touched. |
| `crates/platform-win/src/lib.rs` | Native file dialogs if needed, shortcut capture/registration feedback, notification setup, and existing tray/window behavior. |
| `crates/app/src/regression_tests.rs`, crate tests, `scripts/` | State-transition, migration, failure-injection, UI, performance, and distribution regression evidence. |

Extract settings, task, and recovery coordinators as necessary to keep `main.rs` comprehensible. Do not turn the product work into a wholesale architecture rewrite.

Before changing navigation, replace or centralize hard-coded page indices used by settings loading, report fetching, and display-timer scheduling. Moving a page must not accidentally enable continuous timer rendering on another page or disable reports.

### Migration rules

- Introduce the next schema version transactionally after creating and validating the pre-upgrade backup.
- Use additive task fields for planning/order. Keep existing IDs and history tables intact.
- Set old planning dates to absent; never classify the entire existing backlog as today's work.
- Retain and expose existing soft-deleted tasks in Trash.
- Keep serialized widget preference slots stable even if their configuration page moves.
- Add defaults for new serialized settings fields so existing settings load.
- Older binaries already reject newer schemas. Document that rollback requires the preserved pre-upgrade backup; never promise in-place downgrade.
- Test upgrade fixtures from all currently supported older schema versions and legacy path layouts.
- On migration failure, leave the original data usable and explain the recovery path.

### State and history invariants

1. A started session retains its bound task until it ends, even if its next-task selection changes.
2. Completing, deleting, restoring, or planning a task cannot rewrite historical focus time.
3. A completed focus session earns exactly one completion credit.
4. Reset, skip, and mode changes retain accrued focus as partial time.
5. Pauses, sleep, and time after the last durable crash checkpoint receive no invented credit.
6. Pending work is bounded. Accepted writes are retried; rejected input remains recoverable in the interface.
7. A UI action never claims durable success before storage acknowledgment.
8. No feature requires network access, an account, or usage telemetry.

## Release validation and success criteria

### Automated and integration coverage

Use the repository's existing validation commands and extend their fixtures:

```powershell
cargo fmt --all -- --check
cargo test --workspace --features diagnostics --locked
cargo clippy --workspace --all-targets --features diagnostics --locked -- -D warnings
```

Add focused behavioral tests for:

- Today membership/order/search; old-task migration; carry-forward behavior at midnight and time-zone changes.
- Start from task versus Use next; paused-session binding; completion/deletion of the next task.
- Settings drafts across navigation, failed save, discard, and application exit.
- Delete/Undo/Restore through failures, rapid repeated input, and a pending task edit.
- Periodic checkpoints near expiry, during sleep/resume, after queue saturation, and before abrupt process termination.
- Backup/restore round trips and interrupted replacement at each destructive boundary.
- Report reconciliation when checkpoints create more intervals for the same session.

Run existing integration, failure, UI audit, and packaging checks against the actual release candidate. Update scripts that assume page indices or the old navigation layout. Inspect rendered screenshots; image dimensions alone do not validate layout.

### Desktop and release gates

- No unrecoverable data loss or duplicate completion credit in the exercised migration, restore, and failure-injection scenarios.
- Successful restore drill from both a manual backup and an automatic pre-upgrade backup.
- No unresolved keyboard-access, critical clipping, or invisible-blocked-state issues in the main journeys.
- Real sleep/hibernate, Explorer restart, mixed-DPI movement, monitor removal, notification fallback, shortcut conflicts, and install/upgrade/uninstall pass the manual checklist.
- Preserve the current 12,000,000-byte packaged EXE limit. The 0.2.0 executable is 11,670,016 bytes, leaving 329,984 bytes (2.75%) of headroom; treat further growth as a release risk.
- Apply the companion UI/performance plan's explicit latency, private-memory, working-set, CPU, and visual-state gates. Distinguish proposed targets from newly measured results.
- Compare startup, memory, and tray CPU with the same hardware, fixtures, measurement method, and identified binaries. Investigate a repeatable regression greater than 10% in comparable startup/private-memory measures; allow for noise and explain intentional costs.
- Re-measure tray CPU over a sufficiently long interval to capture periodic checkpoint activity. A zero short sample is not evidence of zero work.
- Benchmark both 5,000-task lists and long history enlarged by periodic checkpoints. Keep slow storage and backup work off the UI thread.
- Report first usable paint separately from the existing native input-idle startup metric.

### Product validation without telemetry

Repeat the same observed tasks with at least five representative users. These are proposed acceptance targets, not existing measured outcomes; none were measured for 0.2.0:

| User task | Target |
|---|---|
| Start first focus session, with or without a task | At least 4 of 5 participants succeed within 30 seconds without assistance. |
| Explain which task is currently tracked and which is next | At least 4 of 5 answer correctly after changing selection during a session. |
| Plan three existing tasks for today | At least 4 of 5 complete the task within one minute. |
| Recover an accidentally deleted task | At least 4 of 5 succeed without opening the data folder. |
| Recover from an injected save failure after storage becomes available | At least 4 of 5 use Retry saving and understand that Resume is a separate action. |
| Navigate away from an edited settings form and return | Every exercised draft remains intact unless explicitly discarded. |

Record observations with participants' permission. Do not introduce hidden analytics to measure these targets. Small usability sessions identify friction; they do not establish retention or market demand.

## Decisions to revisit after the first release

| Candidate | Evidence needed before scheduling |
|---|---|
| Timer ring and mini timer as presets of one widget | Users treat them as interchangeable, and migration preserves existing layouts without surprise. |
| Named focus-duration presets | Users repeatedly change the same duration combinations during normal work. |
| Optional daily focus target | Users ask for planning support and can distinguish a chosen target from a productivity judgment. |
| Task tags, projects, recurring tasks, or due dates | The Today/backlog distinction proves insufficient in observed real task lists. |
| Automated update delivery or distribution signing work | A concrete distribution/support need, an ownership plan, and separate release engineering scope. |
| Sync or integrations | Repeated, specific cross-device or external-workflow demand strong enough to justify changing the offline product boundary. |

The recommended first implementation slice combines shared visual tokens and a refined Timer layout with typed recovery state, preserved settings drafts, explicit current/next task labels, and bounded progress checkpoints. Measure this slice before expanding it across the app; it should demonstrate the product's speed, low memory use, visual quality, and reliable accounting together.

# Orbit 0.2.0 — 20 October 2026

Orbit 0.2 adds a quieter light/dark interface and a complete task-to-focus workflow while retaining the native Rust/Slint software renderer.

- Three primary destinations: Timer, Tasks, and Reports. Widgets, Data, and About live in grouped Settings.
- Today planning and ordering, Unicode task search, Carry forward, Completed, Trash, and timed deletion Undo.
- Timer task picker, quick-add, explicit Start focus / Use next behavior, current/next task context, and Resume wording.
- Coalesced one-minute running checkpoints, persistent blocked-save status, and an explicit Retry saving action.
- Preserved settings drafts, Save/Discard, shortcut recording and disable controls, Windows theme following, and reduced-motion handling.
- Native file selection, verified SQLite snapshots, pre-upgrade backups, and atomic restoration with a pre-restore recovery copy. The timer remains paused after restore; this computer's startup opt-in is retained.
- Custom report ranges, session details, and UTF-8 CSV export with formula-safe task titles. Long chart ranges are grouped for readability; CSV retains daily detail.

Existing databases migrate to schema 4 after an automatic backup. Old task IDs, estimates, completed sessions, partial time, and deleted-task history are retained. Existing tasks remain unplanned. Upgrades preserve the previous light appearance and configured shortcuts; new installations follow Windows and leave the skip shortcut disabled.

Older Orbit versions cannot open schema 4. Keep the pre-upgrade backup if a downgrade is needed. Uninstall retains data. Manual and pre-restore backups are retained; the latest three automatic upgrade backups are retained. Native restore replaces the database through SQLite's backup transaction and refreshes the application in place; a restart is unnecessary.

Performance measurements identify the binary and test scope. See PERFORMANCE.md and RELEASE-VALIDATION.md. No comparative “fastest app” claim is established by these measurements.

# Orbit 0.1.1 — 20 October 2026

Orbit 0.1.1 prioritizes fast startup, low runtime memory, and reliable persistence.

- Ship an uncompressed executable by default; ZIP and installer downloads remain compressed.
- Preserve pending task edits when completion is toggled, and coalesce task snapshots to prevent reply backlogs.
- Apply settings only after storage confirms the write.
- Save the final paused session asynchronously through reserved queue capacity before exit.
- Recover correctly after canceled Windows shutdown and sleep/resume during storage errors.
- Bound historical report reads while preserving unusually long legacy intervals.

Requires Windows 11 22H2 or newer, x64. Use the installer or extract the portable ZIP and run orbit.exe. The executable is unsigned.

Existing databases migrate to schema 3. Back up the data directory while Orbit is closed before upgrading; Orbit 0.1.0 cannot open a migrated database. Uninstall retains user data.

SHA256SUMS.txt contains hashes for the installer and portable ZIP. Validation results and measured executable identities are included in the packaged documentation.


# Orbit 0.1.0

Initial Windows 11 x64 release: three timer modes; tasks and estimates; SQLite settings and history; daily, weekly, and monthly reports; tray controls; configurable global shortcuts; soft alarms and volume; Windows notifications; opt-in autostart; three independently enabled widgets with saved placement, layering, and click-through recovery.

Software rendering is the default, with a DPI-aware Windows manifest and a statically linked CRT. Mica falls back to solid surfaces when unavailable or high contrast is active. The About page includes the standard AboutSlint attribution widget.

Task analytics show focus time, share of total focus, and completed sessions for each task in daily, weekly, and monthly reports. Partial sessions, deleted tasks, and unassigned time retain their history. Report titles are resolved once per task, the tasks widget uses a lightweight daily summary, and startup defers analytics until Reports is opened.

Window movement uses Slint's native WindowMoveArea for the main window and all widgets. This fixes controls becoming unclickable after a drag consumed the mouse-release event.

No accounts, cloud sync, ambient sounds, streaks, heatmaps, focus-lock, automatic updater, or WorkerW desktop attachment are included. Release binaries are unsigned unless a distributor signs them separately.

Memory and reliability audit: bounded storage/native queues and alarm/toast concurrency; atomic task mutation snapshots; deterministic native-thread shutdown; safe native drag dispatch; virtualized task rows; minimized display-timer suspension; coalesced reports and schema-v2 report index/join. Schema v1 databases migrate automatically. See STABILITY.md for the measured stress and heap/handle validation results.

Existing pre-Orbit databases are reopened in place. The installer retains its upgrade identity and migrates an existing startup opt-in. Earlier performance and memory-audit records remain labelled as historical evidence.

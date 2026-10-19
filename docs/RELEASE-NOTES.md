# Orbit 0.1.0

Initial Windows 11 x64 release: three timer modes; tasks and estimates; SQLite settings and history; daily, weekly, and monthly reports; tray controls; configurable global shortcuts; soft alarms and volume; Windows notifications; opt-in autostart; three independently enabled widgets with saved placement, layering, and click-through recovery.

Software rendering is the default, with a DPI-aware Windows manifest and a statically linked CRT. Mica falls back to solid surfaces when unavailable or high contrast is active. Slint attribution must be resolved before distribution; see the README.

Task analytics show focus time, share of total focus, and completed sessions for each task in daily, weekly, and monthly reports. Partial sessions, deleted tasks, and unassigned time retain their history. Report titles are resolved once per task, the tasks widget uses a lightweight daily summary, and startup defers analytics until Reports is opened.

No accounts, cloud sync, ambient sounds, streaks, heatmaps, focus-lock, automatic updater, or WorkerW desktop attachment are included. Release binaries are unsigned unless a distributor signs them separately.

Memory and reliability audit: bounded storage/native queues and alarm/toast concurrency; atomic task mutation snapshots; deterministic native-thread shutdown; safe native drag dispatch; virtualized task rows; minimized display-timer suspension; coalesced reports and schema-v2 report index/join. Schema v1 databases migrate automatically. See STABILITY.md for the measured stress and heap/handle validation results.

Existing pre-Orbit databases are reopened in place. The installer retains its upgrade identity and migrates an existing startup opt-in. Earlier performance and memory-audit records remain labelled as historical evidence.

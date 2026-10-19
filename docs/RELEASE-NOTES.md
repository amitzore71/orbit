# Orbit 0.1.0

Initial Windows 11 x64 release: three timer modes; tasks and estimates; SQLite settings and history; daily, weekly, and monthly reports; tray controls; configurable global shortcuts; soft alarms and volume; Windows notifications; opt-in autostart; three independently enabled widgets with saved placement, layering, and click-through recovery.

Software rendering is the default. The application has a top-level Slint attribution page, a DPI-aware Windows manifest, and a statically linked CRT. Mica is optional and falls back to solid surfaces when Windows cannot enable it or high contrast is active.

No accounts, cloud sync, ambient sounds, streaks, heatmaps, focus-lock, automatic updater, or WorkerW desktop attachment are included. Release binaries are unsigned unless a distributor signs them separately.

Memory and reliability audit: bounded storage/native queues and alarm/toast concurrency; atomic task mutation snapshots; deterministic native-thread shutdown; safe native drag dispatch; virtualized task rows; minimized display-timer suspension; coalesced reports and schema-v2 report index/join. Schema v1 databases migrate automatically. See STABILITY.md for the measured stress and heap/handle validation results.

The application is now named Orbit. Window and tray labels, notification identity, executable metadata, shortcuts, and release filenames use Orbit. Existing databases are reopened in place, and the installer retains its upgrade identity and migrates an existing startup opt-in. Earlier performance and memory-audit records remain labelled as historical evidence.

# Orbit 0.3.1

This release packages the experience upgrade described in EXPERIENCE-UPGRADE.md. It includes the Windows x64 portable ZIP, per-user installer, and SHA-256 checksums. The executable is unsigned. Existing user data and settings are preserved.

The upgrade candidate passed 57 Rust tests, 6 frontend tests, 19 browser journeys, and native WebView2/process checks. The release rebuild changes version metadata to 0.3.1. Package verification checks executable identity, x64 PE/ASLR/NX, native imports, and ZIP equivalence; it does not claim new final-binary performance measurements.

Known limits: the measured candidate used 304.8MiB mean private commit with the main window running, above the 300MiB target. The ten-minute hidden CPU acceptance check remains incomplete. Candidate measurements retain their original executable hash. Hardware sleep/wake, physical mixed-DPI migration, exhaustive Windows high-contrast checks, and clean-machine installation remain manual checks.

The packaging script supports `-SkipMeasurements` to omit new benchmarks explicitly. Verification records `finalBinaryMeasurementsVerified: false`; matching final-binary measurements remain mandatory by default.

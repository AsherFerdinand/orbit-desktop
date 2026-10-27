# Changelog

## 0.5.0 — 27 October 2026

### Added
- Click/drag/keyboard seeking for capable Windows media sessions; unsupported finite timelines remain read-only.
- Optional persistent Quick Note reminders delivered through native Windows notifications.
- Current-user Windows Light/Dark control and ORBIT System appearance following.

### Changed
- Brighter green accent, automatic Brave preference and advanced-settings session selection.
- Read-only volume percentage and compact temporary volume HUD; removed slider/mute UI.
- Default auto-collapse is 400ms with focus, draft and scrubbing protection; saved preferences remain respected.
- Adaptive network sampling, CPU/RAM suppression when unnecessary and immediate opening refresh.
- Artwork IDs/on-demand transfer, shared current covers and a bounded frontend cache.
- Skip unchanged note/settings persistence writes.

### Fixed
- Restored the shared green accent for dark-mode media controls and increased dark-mode timeline track contrast; light-mode colours are unchanged.
- Portable release reminders create/update a per-user AppUserModelID shortcut before delivery, fixing Windows error 0x80070490 without an installer.
- ORBIT's System appearance now updates when the Windows application theme changes.
- Removed duplicate adapter enumeration during expanded system sampling.


## 0.4.0 — 25 October 2026

- Replaced four-tab navigation with one ambient media/system/volume/notes surface.
- Added persistent collapsed session artwork, finite crossfades, music fallback, paused context and optional finite-timeline progress.
- Moved media state to app lifetime; corrected ended-session grace timing and released obsolete failed-artwork references.
- Added locally persisted plain-text notes with stable IDs/timestamps, edit/delete/complete/reopen, validation, previous-file backup and flushed replacement writes.
- Added up to three clickable paper indicators plus additional active-note count.
- Extended native GDI regions and added finite, throttled animation-region updates without per-frame window resizing.
- Preserved draft state under overlays and blocked unwanted collapse during editing/control interactions.
- Added six defaulted preferences and preserved original pre-0.4 settings before migration saves.
- Shared the native tray/context menu and added explicit current-view process-tree sampling.
- Added ambient/notes/migration/geometry tests and actual Windows UI/performance evidence.
- Preserved earlier releases and labelled historical verification separately.


## 0.3.0 — 24 October 2026

- Added native event-driven Windows GSMTC integration, prioritising Brave rather than Spotify.
- Added Media panel, explicit Windows session selection, real metadata/artwork, capability-gated controls and optional timeline.
- Preserved selection across collapse; removed sessions fall back to automatic selection.
- Bounded native callback queue, metadata-only artwork refresh, lightweight revision events, and no collapsed progress timer.
- Added timeout/error handling and a read-only native probe. No browser extension or media notification spam.
- Added media selection/timeline/artwork regression tests. Brave end-to-end verification remains blocked; see MEDIA.md.


## 0.2.0 — 24 October 2026

- Added real CPU, physical RAM, and local adapter status in a dedicated System panel.
- Added default-output Core Audio volume/mute control, external callbacks, and device replacement handling.
- Added Windows power callbacks and transition-based charging/low/critical/full notices.
- Added a central priority queue, deduplication, expiry, coalescing, and interaction-aware notice display.
- Added rounded OS input regions to exclude transparent margins/corners.
- Extended validated preferences with backward-compatible 0.1 defaults and a legacy backup.
- Added user-triggered process-tree performance sampling, including WebView2 children.
- Extended unit/native tests and documented Windows verification and remaining hardware cases.
- Preserved the existing 0.1 release. Portable executable packaging is prioritized over installer tooling.

## 0.1.0

Initial transparent island, real Windows battery readings, local preferences, themes, spring transitions, automatic collapse, and tray controls. The original source archive and executable remain available.

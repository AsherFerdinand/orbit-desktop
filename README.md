# ORBIT 0.5.0 — Your desktop, in motion.

A lightweight Windows Dynamic-Island-style desktop companion built with Tauri 2, React/TypeScript and Rust. Local notes and native Windows integrations; no account, cloud service or browser extension.

## Launch

Download [ORBIT-0.5.0-windows-x64.exe](https://github.com/AsherFerdinand/orbit-desktop/releases/download/v0.5.0/ORBIT-0.5.0-windows-x64.exe) from the [GitHub Release](https://github.com/AsherFerdinand/orbit-desktop/releases/tag/v0.5.0). Exit older ORBIT instances through the tray, then double-click the downloaded executable. Run one version at a time. The unsigned portable Windows x64 executable needs Microsoft Edge WebView2 Runtime. It works without localhost or a development server. Windows 11 is recommended; Windows 10 is also supported.

Hover or click the island to expand it. The default auto-collapse delay is 400ms after leaving; existing saved delays are preserved. Focus, unsaved note drafts and media scrubbing protect active interaction. Minus or Escape collapses when no draft/edit is pending. Right-click a non-text area, use Shift+F10, or open the tray menu for advanced settings, Show/Hide and Exit.

<img width="328" height="63" alt="Orbit Interface 1" src="https://github.com/user-attachments/assets/7165dbb4-41e6-428b-b50a-da0268a09efb" />


_Collapsed island, captured in 0.4. The expanded interface and controls have evolved in 0.5.0._

<img width="560" height="572" alt="Orbit Interface 2" src="https://github.com/user-attachments/assets/5bf5245a-405c-467e-b70c-fa29e356adc8" />


## What's in 0.5.0

- Floating always-on-top island with contextual expansion and interaction protection.
- Native Windows battery/charging, CPU/RAM and local adapter information.
- Refined island interaction, brighter green accent, read-only volume percentage and a brief volume HUD.
- Native GSMTC media metadata, artwork and capability-aware controls, prioritising YouTube/YouTube Music in Brave. Manual Windows-session selection is in advanced settings.
- Seekable expanded media timeline: click, drag or use arrow keys/Home/End when Windows exposes seeking. Reliable finite timelines without seek capability stay read-only; missing timelines are omitted. Collapsed progress is read-only.
- Quick Notes with edit/complete/reopen/delete, paper indicators and optional date/time reminders delivered as native Windows notifications.
- Independent ORBIT System/Light/Dark appearance and current-user Windows Light/Dark controls. No administrator rights are needed.
- Adaptive background sampling, immediate opening refresh and artwork transferred only on identity changes.

## Notes, reminders and preferences

Type a quick note and press Enter or Add. The clock control adds an optional local reminder time. Complete a note to cancel its pending reminder; reopening does not re-enable it automatically. ORBIT must remain running for reminders (hidden is fine). Notification banners depend on Windows settings/Do Not Disturb; notification click-to-open is not implemented. On first reminder delivery, the portable app creates/updates its per-user Start Menu ORBIT shortcut with the notification identity Windows requires. It does not add autostart, enable blocked notifications or require an installer.

Data stays in **%APPDATA%/com.orbit.desktop/**. Existing notes/settings are reused across upgrades. Notes have stable IDs, persisted reminder state and a previous-file backup. Invalid/corrupt files are preserved rather than silently reset. Limits: 500 notes, 500 Unicode characters per note, 2 MiB notes file. Unsaved drafts survive running-app overlays but not process exit. Settings preserve earlier values and legacy backups.

Windows/Brave determines session granularity: a tab is not necessarily a separate session. Missing artwork/metadata/controls cannot be invented. Browser commands may be declined; live/DVR timelines may not describe a complete recording. Artwork comes only from the native session, capped at 2 MiB per thumbnail.

CPU/RAM samples run only while visible compact metrics are active, default every 2s. Network uses one 2s active / 10s collapsed / 30s hidden schedule; battery keeps native power callbacks and 5s visible / 30s hidden fallback. These are local system readings, not external analytics.

## Validation and development

See [0.5 release validation](docs/RELEASE-0.5.0.md), [changelog](docs/CHANGELOG.md), [codebase map](CODEBASE-MAP.md) and [contributor guidance](AGENTS.md). The included screenshot is labelled with its capture version. No RAM/CPU reduction is claimed from development measurements.

Development needs Node.js 22+, Rust stable MSVC, Visual Studio C++ Build Tools/Windows SDK and WebView2.

```powershell
git clone https://github.com/AsherFerdinand/orbit-desktop.git
cd orbit-desktop
npm ci
npm run tauri -- dev
```

For an optimized portable build:

```powershell
npm run tauri -- build --no-bundle
```

The executable is written to src-tauri/target/release/orbit-desktop.exe. Tauri embeds the Vite assets and uses its production custom protocol.

Checks: npm run typecheck, npm test, and cargo test --manifest-path src-tauri/Cargo.toml. The optional orbit.ps1 helper provides dev/build/check tasks and falls back to the system toolchain outside the original workspace. See AGENTS.md before contributing.

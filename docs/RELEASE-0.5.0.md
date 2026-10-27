# ORBIT 0.5.0 release

Portable Windows x64 release. Download the executable from the [GitHub Release](https://github.com/AsherFerdinand/orbit-desktop/releases/tag/v0.5.0). Requires Windows 10 or 11 and Microsoft Edge WebView2 Runtime; Windows 11 is recommended.

## Validation

The source version is consistently 0.5.0 in npm, Cargo and Tauri metadata, including the corresponding lockfile entries.

Prior release validation passed 50 frontend tests and 48 Rust tests, with one explicitly interactive audio test intentionally ignored. The dark-theme media correction subsequently passed TypeScript, formatting, 27 focused media tests and an optimized Tauri build. The maintainer confirmed manual acceptance before publication.

Publication changes are documentation, ignore rules and trailing-whitespace cleanup only; media, reminders, UI behavior and telemetry source are unchanged. A production frontend build and optimized Tauri compilation were run to remove personal build paths from the distributable. No repeated full regression or performance profiling was performed for publication. The executable uses embedded production assets and does not need a development server.

## Distribution and privacy

- The unsigned portable EXE and its SHA-256 checksum are release assets, not Git-tracked binaries. GitHub supplies source archives.
- Rust diagnostic paths are remapped for the public binary. The existing local executable is preserved.
- AppData notes/settings, credentials, logs, build directories, generated Tauri schemas, historical profiling output and note-bearing screenshots are excluded from source history.
- The one published screenshot was inspected and has no notes or account details. It is labelled as a 0.4 capture.
- No software license has been selected for this repository.

## Known limitations

- Run one ORBIT instance/version at a time. ORBIT must remain running for reminders; hiding the island is supported.
- Windows notification settings and Do Not Disturb can suppress banners. Notification click-to-open is not implemented.
- Brave/Windows determines session granularity, metadata, artwork, timeline and playback/seek capabilities. Not every browser tab is separately selectable.
- No consistent CPU/RAM reduction is claimed from development samples. Event-driven integrations, adaptive sampling, bounded artwork caching and fewer redundant transfers/writes describe the implemented behavior.

## Published executable identity

ORBIT-0.5.0-windows-x64.exe: 9,810,432 bytes, optimized Windows x64 build.

SHA-256:

```text
1154ad3c01d68133531e4c10a77f8cb6f3c0749f127bcfb604debf3f0431dddd
```

# ORBIT contributor instructions

## Identity and scope
- ORBIT is a lightweight, single-window Windows 10/11 desktop companion.
- Target laptops include machines with approximately 8 GB RAM.
- Keep it responsive and inexpensive to run continuously.
- Stack: Tauri 2, Rust, React, TypeScript, Vite, Windows APIs and GSMTC.
- Primary media targets are YouTube and YouTube Music in Brave, not Spotify.
- A browser tab is not necessarily an independently exposed Windows media session.
- Document native limitations rather than inventing metadata or controls.

## Before changing code
1. Read this file and CODEBASE-MAP.md.
2. Locate the existing implementation and inspect relevant files first.
3. Reuse existing components, services and event paths.
4. Make the smallest reasonable change that addresses the request.
5. Check affected references before deleting code.

## Responsibility boundaries
- React owns presentation, island state, animations and temporary HUDs.
- React also owns note editing UI and media presentation.
- Rust owns Windows APIs, battery, CPU/RAM, Core Audio and GSMTC.
- Rust owns persistence, native notifications, window behavior and hit testing.
- Tauri commands and events are the frontend/native boundary.
- Keep Windows-specific behavior in Rust unless there is a technical reason otherwise.
- Reuse the native transition detector and frontend notice scheduler.
- Preserve the separate native/system and GSMTC workers.
- Their COM/WinRT objects and callbacks belong to their owning worker threads.
- Clean up callbacks and resources on those threads before apartment teardown.

## Product and interaction
- Keep the island glanceable, contextual, compact, unobtrusive and fast.
- Avoid dashboards, tabs, navigation pages and large menus.
- Avoid unnecessary controls or settings inside the island.
- When nothing relevant is happening, let ORBIT get out of the way.
- Briefly surface useful changes through the existing event architecture.
- Preserve note drafts and editing focus across temporary notices.
- Defer ordinary notices during active interaction; preserve urgent-event priority.
- Advanced settings belong behind the existing native menu.

## Implementation discipline
- Prefer explicit, readable code and the simplest implementation that works.
- Avoid speculative abstractions and unrelated refactoring.
- Do not duplicate state, services or event systems.
- Add dependencies only when the existing stack cannot reasonably do the job.
- Do not add a manager/controller/provider/service for a single-use feature
  unless technically justified.
- Do not split large files merely to reduce their line counts.
- Preserve required compatibility logic and historical documentation.
- Keep TypeScript DTOs/defaults aligned with Rust serialization and validation.
- Do not redesign media transport or native geometry as incidental cleanup.

## Continuous-runtime performance
- Prefer native callbacks/events over high-frequency polling.
- Where polling is necessary, use the slowest reasonable interval.
- Stop or reduce work when a module is disabled or unnecessary where practical.
- Inspect existing timers before adding another timer or listener.
- Avoid duplicate artwork copies, unnecessary renders and persistent large objects.
- Avoid additional WebView windows unless the task requires them.
- Clean up timers, subscriptions, event listeners and native resources.
- Preserve thread ownership and event coalescing when changing callbacks.
- Do not run performance profiling unless explicitly requested.

## Verification
- Prioritise behavior that is hard to verify visually: persistence, migrations,
  calculations, scheduling, priority, deduplication, state transitions,
  native conversions, error handling and data integrity.
- Do not maximise test count or mirror implementation details with tests.
- Do not add excessive tests for colours, spacing, icons, layout, hover feel
  or animation aesthetics; the user manually evaluates interaction quality.
- Run focused tests while developing.
- Run the existing regression suite once after a meaningful change if practical.
- Do not repeatedly run production builds for a small edit.
- Report exactly which checks ran and which manual checks remain.

## Commands (from repository root)
- Frontend development: npm.cmd run dev
- TypeScript check: npm.cmd run typecheck
- Focused test: npm.cmd test -- tests/events.test.ts
- Frontend regression: npm.cmd test
- Native check: cargo check --offline --manifest-path src-tauri/Cargo.toml
- Native regression: cargo test --offline --manifest-path src-tauri/Cargo.toml
- Rust formatting: cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
- Rust tests require Windows; the interactive audio test is intentionally ignored.
- A local Rust toolchain may live under ../../work/toolchain; inspect the
  environment before assuming cargo is on PATH.
- Inspect orbit.ps1 before using it: its check command includes a frontend build.

## Release boundaries
- Normal development: implementation, focused checks, development preview
  where useful, then user manual testing.
- Package an EXE/installer only when a release is explicitly requested.
- Do not automatically create source ZIPs or regenerate screenshots.
- Do not rewrite all documentation as part of ordinary development.
- Do not change the application version during architecture maintenance.
- Do not run hardware acceptance exercises unless requested or necessary.

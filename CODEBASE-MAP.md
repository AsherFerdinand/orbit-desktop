# ORBIT codebase map

Navigation map for the current 0.5 architecture. Read AGENTS.md before editing.
Paths below are relative to this repository root. Generated dist/, node_modules/,
src-tauri/target/ and src-tauri/gen/ are not source navigation targets.

## Frontend entry points and components
| File | Responsibility |
|---|---|
| src/main.tsx | React StrictMode entry and global stylesheet import. |
| src/App.tsx | Single island composition, collapsed/expanded presentation, focus/pointer routing, measured content height, native menu and animation/region coordination. |
| src/components/Ambient.tsx | Artwork fallback/crossfade, playback progress interpolation/local scrub preview, release-to-commit seeking and up-to-three animated note papers. |
| src/components/Icons.tsx | Inline SVG icons. |
| src/styles/orbit.css | Shared theme, island, ambient media, notes, volume, notices and settings styles; kept together deliberately. |
| src/types/index.ts | Settings/defaults, battery/snapshot/volume DTOs and display labels; align with Rust. |

## Hooks, active modules and services
| File | Responsibility |
|---|---|
| src/hooks/useOrbit.ts | Settings and system snapshot, island visibility/view, interaction/collapse guards, notice scheduling, monitoring activity, serialized geometry/settings requests and explicit diagnostics. |
| src/hooks/useMedia.ts | Subscribe/read revisioned media cache, coalesce overlapping reads, select Windows session, hydrate selected artwork through a four-entry promise cache, retain transient display and dispatch controls. |
| src/hooks/useNotes.ts | Subscribe/load notes and serialize mutations/refreshes; owns persisted note list, reminder fields, busy/error and active count. |
| src/hooks/useSystemTheme.ts | In System mode, observe WebView2 prefers-color-scheme changes and resolve the island colour; no polling. |
| src/hooks/useIslandRegion.ts | Measure animated DOM body/papers and send deduplicated, throttled native region updates. |
| src/modules/AmbientMedia.tsx | Active media metadata, artwork, controls and timeline; automatic session selection. |
| src/modules/Notes.tsx | Draft/edit text, completed filtering and note actions; reports editing guard to useOrbit. |
| src/modules/Settings.tsx | Advanced preferences, independent ORBIT/Windows appearance controls, manual Windows-session override and explicit footprint diagnostic. |
| src/services/native.ts | Tauri wrappers, event subscriptions and browser-preview fallbacks for system/settings/window functions. |
| src/services/media.ts | Media DTOs, bounded artwork request cache, session preference, timeline validity/interpolation, paper-count and collapse policies. |
| src/services/notes.ts | Local datetime input / absolute Unix-millisecond conversion and validation. |
| src/services/events.ts | Deterministic bounded notice queue: priority, deduplication, volume coalescing, expiry and interaction deferral; also view/height compatibility decisions. |

The unified panel stays mounted while hidden to preserve note drafts. Media/notes/
region hooks invoke some commands directly; native.ts is a partial facade.
The retired VolumePanel slider/mute UI and frontend volume-write wrappers are
removed; native Core Audio monitoring, callbacks and set_volume remain. The HUD
uses the existing EventManager and window. Hidden note papers are excluded from
measured hit regions; pointer enter/leave covers the complete outer shell.
Legacy MediaPanel.tsx and SystemPanel.tsx were confirmed unreferenced and removed,
including their standalone styles and MediaPanel's inactive progress timer.
Legacy view/height aliases remain for compatibility; the current UI opens ambient
or settings rather than those old standalone panels.

## Rust/native modules
| File | Responsibility |
|---|---|
| src-tauri/src/main.rs | Windows application entry into orbit_lib::run. |
| src-tauri/src/lib.rs | Tauri command registration, state/setup, tray/menu, initial window and worker-stop dispatch. |
| src-tauri/src/runtime.rs | Native/system worker, snapshot cache, activity-aware deadlines, audio/power messages and native transition emission. |
| src-tauri/src/audio.rs | Core Audio device/endpoint ownership, callbacks, generation filtering and validated master-volume writes. |
| src-tauri/src/battery.rs | GetSystemPowerStatus conversion, including unknown/unavailable fields. |
| src-tauri/src/power.rs | Power-setting callback registration for AC/DC and battery percentage; unregister on drop. |
| src-tauri/src/system.rs | GetSystemTimes CPU delta, GlobalMemoryStatusEx RAM and local adapter connectivity. |
| src-tauri/src/media.rs | GSMTC manager/session callbacks, metadata/artwork/timeline cache, session IDs, capability-checked controls/seeking and timeouts. |
| src-tauri/src/events.rs | Detect battery/charger/network transitions and construct notice payloads; frontend scheduling stays in events.ts. |
| src-tauri/src/notes.rs | Validated note document, optional reminders, durable claims, nearest deadline, process/shared Windows mutex, backup and durable replacement. |
| src-tauri/src/reminders.rs | Native WinRT toast XML/delivery on orbit-native; portable current-user notification identity, development fallback, no extra worker. |
| src-tauri/src/settings.rs | Defaults/range validation, JSON persistence, legacy backups and worker settings updates. |
| src-tauri/src/theme.rs | On-demand HKCU theme read/write, consistent app/system DWORDs, rollback/read-back and WM_SETTINGCHANGE broadcast. |
| src-tauri/src/window.rs | DPI/work-area placement, outer bounds, GDI body/paper regions, native hit-test diagnostic and WebView memory hint. |
| src-tauri/src/native_menu.rs | Shared tray/context menu actions: show, settings, measurement, hide and exit. |
| src-tauri/src/performance.rs | Explicit process-tree CPU/memory and optional hit-test measurement; no continuous profiling. |
| src-tauri/examples/media_probe.rs | Standalone read-only Windows media-session diagnostic. |

## Feature ownership
| Feature | Frontend | Native |
|---|---|---|
| Island / collapsed-expanded state | App, useOrbit, orbit.css | window.rs |
| Media / session choice / controls | useMedia, AmbientMedia, services/media | media.rs |
| Artwork | useMedia selected-cover cache, Ambient.Artwork | media.rs current-cover Arc/ID, thumbnail read and on-demand payload command |
| Playback timeline | Ambient.Progress, services/media | media.rs GSMTC timeline |
| Quick notes / persistence | Notes, useNotes, services/notes | notes.rs |
| Reminders / Windows toast | Notes clock/time fields; useNotes refresh | runtime.rs deadline, notes.rs durable claim, reminders.rs native toast |
| Battery / charging | App, useOrbit, types | battery.rs, power.rs, runtime.rs |
| CPU/RAM | App compact metrics, useOrbit activity | system.rs, runtime.rs |
| Master volume / compact HUD | App read-only indicator, useOrbit, services/events | audio.rs, runtime.rs callbacks |
| Notices / priority | useOrbit, services/events | events.rs, runtime.rs |
| Settings | Settings, useOrbit, types | settings.rs |
| Windows appearance | Settings.WindowsAppearance, services/native | theme.rs |
| Native hit region | useIslandRegion, App animation callbacks | window.rs |
| Tray / native context menu | App menu request | lib.rs, native_menu.rs |

## Worker ownership and shutdown
- orbit-native is a dedicated COM MTA thread. It owns Core Audio objects and power
  registration, receives an unbounded mpsc message stream, samples sensors and
  publishes a mutex-protected Snapshot. Its recv_timeout sleeps to the next deadline.
- orbit-media is a separate WinRT MTA thread with a current-thread Tokio runtime.
  It owns the GSMTC manager, session objects and event tokens. Callbacks enqueue
  Wake with try_send on a bounded 64-message channel; metadata callbacks also mark
  dirty state. It blocks on messages, with no recurring media enumeration poll.
- Keep these workers separate: independent blocking WinRT operations and COM
  object/callback lifetimes should remain isolated from system/audio work.
- Native worker drops audio/power registrations before CoUninitialize. Media session
  Drop unregisters handlers; normal worker exit removes manager tokens before
  RoUninitialize. lib.rs sends Stop on Exit (media uses try_send). Thread handles
  are not retained/joined; a full media queue can reject that stop request.

## Data and control flow / IPC
```text
Windows callbacks / sensor reads
  -> owning Rust worker -> cached snapshot / transition / media revision
  -> Tauri event -> React hook -> presentation
React interaction -> Tauri command -> Rust validation / worker message
  -> Windows API -> callback or refreshed cache -> event -> React
Notes/settings actions -> Rust validation -> JSON persistence -> returned state
```
| Commands / events | Owner / consumers |
|---|---|
| get_snapshot, set_monitoring, set_volume | runtime.rs; native.ts/useOrbit (set_volume retained for native compatibility only) |
| get_battery | battery.rs; retained backend command, no frontend wrapper |
| get_media, get_media_artwork, media_action, media_seek | media.rs; useMedia (controls routed to media worker) |
| list_notes, mutate_note | notes.rs; useNotes |
| load_settings, save_settings | settings.rs; native.ts/useOrbit |
| get_windows_theme, set_windows_theme | theme.rs; native.ts/Settings.WindowsAppearance; not persisted in ORBIT preferences |
| resize_island, set_island_region, hide_island | window.rs; useOrbit/useIslandRegion/native.ts |
| show_system_menu | native_menu.rs; App through native.ts |
| measure_performance | performance.rs; useOrbit explicit diagnostic |
| orbit-snapshot | Changed system snapshot; runtime -> useOrbit. |
| orbit-event | System transition -> useOrbit -> EventManager; NOTE_REMINDER_DUE (priority 90) is native-only and skipped by island notices. |
| orbit-notes | Notes mutation/retired occurrence -> useNotes serialized refresh; no polling. |
| orbit-media | Revision only; media -> useMedia -> get_media small cached state (artworkId, no image bytes). |
| orbit-visible, open-settings, measure-current | Native menu/window -> useOrbit. |

Subscribe before initial cache reads to avoid startup races. Media IDs identify
exposed Windows sessions, not browser tabs. Selection prefers explicit choice,
playing Brave, another Brave session, Windows current, playing other, then first.
Only native metadata/artwork/controls/timeline actually exposed are presented.
Artwork: discovery/metadata callbacks alone read/encode the capped 2 MiB thumbnail. Equal
current image data keeps its artworkId and Arc; playback/timeline clones share
that allocation. Native retains one current cover per exposed Windows session,
with no song history; closed sessions release it. get_media and seek replies omit
image bytes. useMedia requests only the selected cover via get_media_artwork(id),
coalesces requests and retains at most four recent promise results (including
failure/null), plus the displayed item/grace. ID changes cancel stale presentation
updates. There is no artwork retry timer; unchanged playback never refetches it.
Seeking: expanded Ambient.Progress previews pointer position locally with capture;
release calls useMedia.seek -> media_seek(id, relative seconds) -> orbit-media
Message::Seek -> TryChangePlaybackPositionAsync(absolute 100ns ticks). The existing
seekable field now combines IsPlaybackPositionEnabled, valid duration and usable
MinSeekTime/MaxSeekTime. Valid timelines without capability stay read-only; unknown
ones stay hidden. The worker rechecks bounds/session, clamps, performs one operation,
then refreshes/publishes GSMTC state before replying with that state. Failure clears
preview and logs locally, without a notice. Session/track identity changes unmount
the old scrub preview. The collapsed strip remains read-only. useOrbit's existing
collapse scheduler also checks a media-interaction ref through drag and commit;
notes keep their independent editing guard. No new recurring timer or worker.
Notice queue: 16 entries, stable equal-priority FIFO; priority >=85 bypasses
interaction deferral and can preempt a lower-priority notice. Non-volume dedup is
30s; pending expiry is 30s (volume 1s). Volume HUD lifetime is 1s from the latest update;
expanded or active interaction suppresses new volume notices. The HUD replaces only
collapsed presentation without changing the underlying view, notes or media state.
App excludes volume events from generic notice markup and its 220px height. Incoming push, tick and dismissal use the
same pointer/focus/editing/settings guard; navigation deliberately holds the queue.

## Note reminders
- notes.json stays version 1. reminderAt is nullable Unix milliseconds; reminderEnabled
  and reminderFired default false, so old notes load as ordinary notes. datetime-local
  is parsed/displayed with standard local Date facilities; no stored local-time string.
- Create/edit can set, disable or remove a reminder. A changed future time resets fired
  state. Text-only edits preserve reminder state even if it fires while editing.
  Completion disables it; reopening does not re-enable it. Deletion removes it.
- The existing orbit-native worker tracks only the nearest absolute deadline and
  includes it in recv_timeout. Mutations send NotesChanged(next deadline). Startup
  reloads pending deadlines. No extra thread, per-note timer or reminder polling.
- Due claims re-read under NotesLock plus Local\ORBIT.Notes.v1 Windows mutex, save
  fired=true/enabled=false durably, then attempt delivery. This also prevents duplicate
  claims by multiple current ORBIT instances. Failed persistence does not notify and
  retries after 30s; failed toast delivery is logged and is not automatically retried.
- Overdue <=15 minutes: attempt once. Older: retire quietly. A crash after the durable
  claim but before delivery can lose a notification; this deliberately favours no spam.
- reminders.rs uses existing windows crate UI_Notifications/Data_Xml_Dom bindings:
  RoInitialize -> XmlDocument/CreateTextNode -> ToastNotification -> ToastNotifier.Show.
  Note text is XML text, not markup. Native code needs no frontend notification plugin
  or additional Tauri permissions. No fake reminder window is created.
- Development uses the standard PowerShell AUMID fallback, so Windows may identify
  the source as PowerShell; the content title is ORBIT. Installed builds use the ORBIT
  identifier/AUMID. On first reminder delivery per process, a portable release
  creates/updates the current user's Start Menu Programs/ORBIT/ORBIT.lnk with the
  current executable path and System.AppUserModel.ID=com.orbit.desktop. Windows
  requires this shortcut identity for desktop toast delivery. Existing windows
  crate Shell/PropertiesSystem/Variant bindings are used; no dependency, installer,
  autostart, permission override or COM activation server is added.
- NOTE_REMINDER_DUE uses existing Event at priority 90 (above status, below critical
  battery 100), but does not force island expansion or interrupt editing/scrubbing.
  orbit-notes refreshes note state independently of window visibility.
- ORBIT must remain running (hidden is fine). Windows notification settings/Do Not
  Disturb can suppress banners. No Task Scheduler, startup service or reliable
  notification-click activation is provided. Paper indicators are unchanged; saved
  notes show a subtle clock with the local due time in its tooltip.

## Windows appearance
Settings.WindowsAppearance -> get_windows_theme / set_windows_theme -> theme.rs ->
HKCU/Software/Microsoft/Windows/CurrentVersion/Themes/Personalize. Both
AppsUseLightTheme and SystemUsesLightTheme are REG_DWORD: 0 dark / 1 light.
Only explicit user selection writes both; startup/settings reads never apply a
saved Windows value. Missing/malformed values show Unknown; split preferences
show Mixed. ORBIT appearance offers System, Light and Dark. System follows the
Windows application colour preference through WebView2 prefers-color-scheme and a
change listener; explicit Light/Dark remain independent overrides. New/default
settings use System, while existing saved manual choices are preserved. Only the
ORBIT appearance choice is saved; no Windows preference is restored from disk.

Writes use RegCreateKeyExW/RegSetValueExW with current-user query/set permissions,
then HWND_BROADCAST WM_SETTINGCHANGE with ImmersiveColorSet via
SendMessageTimeoutW (100ms per recipient, SMTO_ABORTIFHUNG). The on-demand write
command uses Tauri's existing blocking pool, not a dedicated theme worker or the
sensor/reminder worker. Failure of the second write attempts to restore the first
value (including its original type or absence). Native read-back returns actual
state; frontend write errors trigger another read. Refresh/broadcast failures are
logged and shown inline, not as island notices. Compatible applications decide
how/when to apply the preference; no Explorer restart, elevation or HKLM write.

The control mounts only while advanced settings are visible. It reads on mount,
window focus and document visibility restoration, and removes listeners on
unmount. Request generations reject stale replies. There is no registry watcher,
recurring timer or polling, and the saved settings schema is unchanged.

## Existing timers, listeners and callbacks
| Owner | Trigger / interval / cleanup |
|---|---|
| runtime CPU/RAM | Default 2s, validated 1-10s; only visible, requested active and system enabled. useOrbit requests active for ambient/system with compactMetrics and no notice; collapsed/hidden CPU/RAM sampling remains disabled. useOrbit depends on notice presence, not changing notice payload objects. |
| runtime network | One schedule: active visible metrics default 2s (settings 1-10s), otherwise visible 10s, hidden 30s; no duplicate adapter enumeration inside CPU/RAM reads. Interfaces are rediscovered each scheduled read, not cached indefinitely. |
| runtime battery | 5s visible / 30s hidden fallback; power callbacks force immediate reads. Hidden-to-visible or inactive-to-active metrics refresh battery/network and CPU/RAM immediately; CPU baseline resets (unavailable until next fresh delta), never averages over the hidden interval. Repeated unchanged activity/settings do not force sensor reads. |
| runtime reminders | One nearest absolute deadline in the existing recv_timeout; idle when none; recalculated on note mutations/startup/due claims. Existing 5/30s system wakes also re-evaluate wall-clock changes/resume (up to 30s correction latency); no reminder-specific filesystem poll. |
| useSystemTheme | WebView2 prefers-color-scheme change listener only in System mode; removed for manual Light/Dark or unmount. No timer. |
| Settings.WindowsAppearance | On-demand read while visible settings mount; window focus / document visibilitychange refresh; listeners removed on unmount. No timer. |
| runtime audio | Endpoint-volume/default-device callbacks; own-write GUID and generation filtering; recovery deadline every 30s, no normal volume poll. |
| media.rs | Manager sessions/current-session and per-session metadata/playback/timeline callbacks. Reads session state per message, metadata/art only when dirty. WinRT waits 2s, media reply timeout 4s; audio reply timeout 3s. |
| Ambient.Progress | 1s interpolation tick only while visible/active, playing and reliable timeline; cleared on dependency change/unmount. Multiple visible progress instances each own a tick. |
| useMedia | 2s stopped/removed display grace; cancelled if live media returns. Event subscription removed on unmount. |
| useOrbit hover/collapse | Hover 180ms; collapse default 0.4s (validated 0.1-20s; seconds converted once at scheduling), blocked by pointer, focus, dirty edits, media scrubbing/commit or profiling. Timers cleared on teardown. |
| useOrbit notices | One-shot current deadline, or 30s queued-expiry wake; resync on events/view/pointer changes. |
| useOrbit native bounds | Expand/HUD entry delay 0; ordinary shrink 500ms / animationSpeed; serialized IPC. HUD explicitly requests collapsed bounds and no paper regions. Paper-region count removal waits 180ms (0 if note animation disabled). |
| App geometry | ResizeObserver for unified content; Motion update/completion callbacks; immediate plus 200ms region resync on relevant changes. |
| useIslandRegion | Animation-frame scheduling, minimum 32ms send spacing/retry, single in-flight IPC and identical-shape suppression. Native-window resize listener; no idle geometry poll. |
| useOrbit listeners | orbit-snapshot, orbit-event, orbit-visible, open-settings, measure-current; window blur clears focus/pointer; App pointerup/pointercancel release press protection, preserving keyboard/input focus. Subscriptions/listeners released on teardown. |
| Explicit performance diagnostic | On request only: 2s settle + 20s process-tree sample; never a background timer. |

## Persistence, configuration and tooling
- notes.json and settings.json use Tauri's app config directory, normally
  %APPDATA%/com.orbit.desktop/. Notes enforce version/schema/unique IDs, 500 notes,
  500 Unicode characters per note and a 2 MiB read limit. Mutation reloads and
  replaces the whole document for meaningful mutations; unchanged edit/completion requests preserve timestamps and skip writes. Prior .bak is retained and corrupt data is rejected.
- Settings validate ranges, apply serde defaults and preserve pre-0.2/pre-0.4
  backups as settings-v0.1.json/settings-v0.3.json. Byte-identical saves skip replacement/backups; frontend saves are serialized. Collapse delay remains seconds, now f64
  with a 0.4s default; old integer JSON delays deserialize unchanged (including
  the previous 5s default). Users can adjust a saved delay in advanced settings.
- src-tauri/tauri.conf.json: one transparent, frameless, always-on-top window,
  taskbar exclusion and CSP. capabilities/default.json scopes core event access.
- package.json/package-lock.json: scripts and frontend dependencies; tsconfig.json,
  vite.config.ts and index.html configure frontend entry/build/test tooling.
- src-tauri/Cargo.toml/Cargo.lock/build.rs: native dependencies and Tauri build setup.
  icons/ contains application assets. orbit.ps1 wraps tooling; inspect before use.
- React/ReactDOM render UI; motion animates; @tauri-apps/api bridges IPC. Vite,
  TypeScript, Vitest, React types/plugin, Prettier and Tauri CLI are development tools.
- Rust: tauri supplies window/tray/IPC; serde/serde_json persistence/DTOs; tokio
  runtime/timeouts/oneshot; windows/windows-core typed COM/WinRT and macros;
  windows-sys raw Win32; webview2-com memory hints; base64 artwork; tauri-build setup.
- Development launch: powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\orbit.ps1 -Task dev
  from this root. This starts Vite on 127.0.0.1:1420 and compiles/runs the debug
  application against live source. Existing releases/*.exe and dist/ are independent
  older artifacts; tests/typecheck do not refresh those executables or bundles.
- README.md describes current usage; docs/CHANGELOG.md and docs/RELEASE-0.5.0.md
  describe the current release. Historical profiling files/reports and other captures
  remain local and ignored. The published screenshot is labelled with its capture version.

## Tests
Vitest tests/ cover ambient SSR/policies, battery labels, event scheduling, media
selection/timeline, interaction guards and Windows appearance refresh/error handling (50 cases, including artwork request dedup/failure/bounds).
Rust inline cfg(test) modules cover conversions, math, transitions, settings,
notes, regions, Windows theme interpretation and native basics: 49 tests, including one ignored interactive
volume test (48 regular). The notification identity test creates/reloads/removes
its own shortcut under target/, including a real AppUserModelID property round trip. Optimization coverage includes telemetry wake policy,
artwork identity/shared ownership/serialization and no-op persistence. Tests may create temporary files under target/.
See AGENTS.md for focused/regression commands; a test run is not native Brave
acceptance verification or a release build.

## Potential Future Optimisations
Documented candidates only; not implemented by this cleanup:
- Measure an optimized release separately; development WebView2 working sets and
  short CPU samples are noisy. Preserve animation behavior while investigating
  render/geometry wakeups if sustained release profiling warrants it.
- Native adapter callbacks/cached discovery would add lifecycle complexity; the
  single 2/10/30s network schedule currently rediscovers interfaces safely.
- Coalesce superseded meaningful settings writes only if measurements justify it;
  keep note durability, reminder claims and validation intact.
- Clarify ownership between outer-window resize/static region and measured animated
  DOM/GDI region; constants currently appear in CSS/React/Rust.
- useOrbit, App, orbit.css, media.rs and window.rs hold several related concerns.
  Extract only for a demonstrated feature need; no size-driven split is required.

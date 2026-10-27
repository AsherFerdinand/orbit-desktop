import { useCallback, useEffect, useRef, useState } from "react";
import {
  native,
  getWindowsTheme,
  setWindowsTheme,
  type WindowsTheme,
} from "../services/native";
import type { useMedia } from "../hooks/useMedia";
import type { Settings as Preferences } from "../types";
export function Settings({
  value,
  change,
  measure,
  performanceResult,
  media,
  active = false,
}: {
  value: Preferences;
  change: (s: Preferences) => void;
  measure: () => void;
  performanceResult: string;
  media: ReturnType<typeof useMedia>;
  active?: boolean;
}) {
  const update = <K extends keyof Preferences>(key: K, val: Preferences[K]) =>
    change({ ...value, [key]: val });
  return (
    <div className="preferences">
      <div className="setting">
        <span>
          ORBIT appearance
          <small>System follows Windows; Light/Dark override it</small>
        </span>
        <select
          aria-label="ORBIT appearance"
          value={value.theme}
          onChange={(e) =>
            update("theme", e.target.value as Preferences["theme"])
          }
        >
          <option value="system">System</option>
          <option value="dark">Dark</option>
          <option value="light">Light</option>
        </select>
      </div>
      {active && <WindowsAppearance />}
      <label className="setting">
        <span>
          Panel width<small>{value.width} px</small>
        </span>
        <input
          aria-label="Panel width"
          type="range"
          min="360"
          max="480"
          step="10"
          value={value.width}
          onChange={(e) => update("width", +e.target.value)}
        />
      </label>
      <label className="setting">
        <span>
          Pill height<small>{value.height} px</small>
        </span>
        <input
          aria-label="Pill height"
          type="range"
          min="48"
          max="64"
          step="2"
          value={value.height}
          onChange={(e) => update("height", +e.target.value)}
        />
      </label>
      <label className="setting">
        <span>
          Auto-collapse
          <small>{value.collapseDelay} seconds after leaving</small>
        </span>
        <input
          aria-label="Collapse delay"
          type="range"
          min="0.1"
          max="20"
          step="0.1"
          value={value.collapseDelay}
          onChange={(e) => update("collapseDelay", +e.target.value)}
        />
      </label>
      <div className="setting">
        <span>Animation speed</span>
        <select
          aria-label="Animation speed"
          value={value.animationSpeed}
          onChange={(e) => update("animationSpeed", +e.target.value)}
        >
          <option value="0.75">Relaxed</option>
          <option value="1">Natural</option>
          <option value="1.5">Quick</option>
        </select>
      </div>
      <label className="setting">
        <span>Battery module</span>
        <input
          className="switch"
          aria-label="Battery module"
          type="checkbox"
          checked={value.batteryEnabled}
          onChange={(e) => update("batteryEnabled", e.target.checked)}
        />
      </label>
      {(
        [
          ["collapsedArtwork", "Collapsed media artwork"],
          ["playbackProgress", "Playback progress"],
          ["quickNotes", "Quick notes"],
          ["paperStack", "Paper stack indicators"],
          ["noteAnimations", "Note animations"],
          ["compactMetrics", "Compact system metrics"],
          ["systemEnabled", "System monitoring"],
          ["volumeNotifications", "Volume notifications"],
          ["chargingNotifications", "Battery & charging notices"],
          ["networkNotifications", "Network notifications"],
        ] as const
      ).map(([key, label]) => (
        <label className="setting" key={key}>
          <span>{label}</span>
          <input
            className="switch"
            type="checkbox"
            aria-label={label}
            checked={value[key]}
            onChange={(e) => update(key, e.target.checked)}
          />
        </label>
      ))}
      {(
        [
          ["refreshInterval", "System refresh", 1, 10, "s"],
          ["notificationDuration", "Notice duration", 2, 10, "s"],
          ["lowBatteryThreshold", "Low battery threshold", 15, 40, "%"],
        ] as const
      ).map(([key, label, min, max, unit]) => (
        <label className="setting" key={key}>
          <span>
            {label}
            <small>
              {value[key]}
              {unit}
            </small>
          </span>
          <input
            type="range"
            aria-label={label}
            min={min}
            max={max}
            value={value[key]}
            onChange={(e) => update(key, +e.target.value)}
          />
        </label>
      ))}
      <label className="setting">
        <span>
          Media session
          <small>Windows sessions; browser tabs may be grouped</small>
        </span>
        <select
          aria-label="Media session"
          value={media.selected ?? "auto"}
          onChange={(e) =>
            media.select(
              e.target.value === "auto" ? null : Number(e.target.value),
            )
          }
        >
          <option value="auto">Auto · Brave</option>
          {media.data.sessions.map((session) => (
            <option key={session.id} value={session.id}>
              {session.source || "Windows media"} ·{" "}
              {session.title || "Untitled"} · #{session.id}
            </option>
          ))}
        </select>
      </label>
      <button
        className="diagnostic-button"
        onClick={measure}
        disabled={performanceResult.startsWith("Sampling")}
      >
        Measure idle footprint (20s)
      </button>
      <p className="footnote" role="status">
        {performanceResult}
      </p>
      <p className="footnote">
        Saved on this device · No account. No telemetry.
      </p>
    </div>
  );
}

export function WindowsAppearance() {
  const [mode, setMode] = useState<WindowsTheme | null>(null),
    [busy, setBusy] = useState(native),
    [error, setError] = useState("");
  const mounted = useRef(false),
    writing = useRef(false),
    sequence = useRef(0);
  const refresh = useCallback(async () => {
    if (writing.current) return;
    const request = ++sequence.current;
    try {
      const actual = await getWindowsTheme();
      if (mounted.current && request === sequence.current) {
        setMode(actual);
        setError("");
      }
    } catch (e) {
      console.warn("ORBIT Windows appearance read:", e);
      if (mounted.current && request === sequence.current) {
        setMode(null);
        setError(String(e));
      }
    } finally {
      if (mounted.current && request === sequence.current) setBusy(false);
    }
  }, []);
  useEffect(() => {
    mounted.current = true;
    void refresh();
    const focus = () => void refresh();
    const visible = () => {
      if (document.visibilityState === "visible") void refresh();
    };
    window.addEventListener("focus", focus);
    document.addEventListener("visibilitychange", visible);
    return () => {
      mounted.current = false;
      sequence.current++;
      window.removeEventListener("focus", focus);
      document.removeEventListener("visibilitychange", visible);
    };
  }, [refresh]);
  async function change(mode: "light" | "dark") {
    if (!native || writing.current) return;
    writing.current = true;
    const request = ++sequence.current;
    setBusy(true);
    setError("");
    try {
      const result = await setWindowsTheme(mode);
      if (mounted.current && request === sequence.current) {
        setMode(result.mode);
        setError(result.warning ?? "");
      }
    } catch (e) {
      console.warn("ORBIT Windows appearance write:", e);
      let actual: WindowsTheme | null = null;
      try {
        actual = await getWindowsTheme();
      } catch (readError) {
        console.warn("ORBIT Windows appearance refresh:", readError);
      }
      if (mounted.current && request === sequence.current) {
        setMode(actual);
        setError(String(e));
      }
    } finally {
      writing.current = false;
      if (mounted.current && request === sequence.current) setBusy(false);
    }
  }
  return (
    <>
      <label className="setting">
        <span>
          Windows appearance<small>Changes Windows app/system appearance</small>
        </span>
        <select
          aria-label="Windows appearance"
          value={mode ?? "unknown"}
          disabled={!native || busy}
          onChange={(e) => {
            if (e.target.value === "light" || e.target.value === "dark")
              void change(e.target.value);
          }}
        >
          <option value="unknown" disabled>
            {busy ? "Reading…" : "Unknown"}
          </option>
          <option value="mixed" disabled>
            Mixed (apps/system)
          </option>
          <option value="light">Light</option>
          <option value="dark">Dark</option>
        </select>
      </label>
      {error && (
        <p className="inline-error" role="status">
          {error}
        </p>
      )}
    </>
  );
}

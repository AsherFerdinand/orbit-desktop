import { useEffect, useState, useRef, useCallback } from "react";
import { defaults, type Settings, type Battery, type Snapshot } from "../types";
import * as api from "../services/native";
import { canCollapse, paperCount } from "../services/media";
import {
  EventManager,
  panelHeight,
  type OrbitEvent,
  type View,
} from "../services/events";
export function useOrbit(
  activeNotes = 0,
  hasMedia = false,
  hasNotes = false,
  measuredHeight: number | null = null,
) {
  const profileRef = useRef<() => void>(() => {}),
    profiling = useRef(false);
  const focusRef = useRef(false),
    dirtyRef = useRef(false),
    mediaInteraction = useRef(false),
    pointerRef = useRef(false);
  const [performanceResult, setPerformanceResult] = useState("");
  const [settings, setSettings] = useState(defaults),
    [snapshot, setSnapshot] = useState<Snapshot | null>(null),
    [view, setView] = useState<View>("closed"),
    [error, setError] = useState(""),
    [ready, setReady] = useState(false),
    [visible, setVisible] = useState(true),
    [notification, setNotification] = useState<OrbitEvent | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined),
    hoverTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined),
    eventTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const manager = useRef(new EventManager()),
    interacting = useRef(false),
    viewRef = useRef(view),
    settingsRef = useRef(settings),
    visibleRef = useRef(visible),
    resizeChain = useRef(Promise.resolve()),
    saveChain = useRef(Promise.resolve());
  viewRef.current = view;
  settingsRef.current = settings;
  visibleRef.current = visible;
  const report = useCallback((e: unknown) => setError(String(e)), []);
  const isInteracting = useCallback(
    () =>
      interacting.current ||
      focusRef.current ||
      dirtyRef.current ||
      mediaInteraction.current ||
      viewRef.current === "settings",
    [],
  );
  const syncEvents = useCallback(() => {
    clearTimeout(eventTimer.current);
    const q = manager.current;
    const event = q.tick(performance.now(), isInteracting());
    setNotification(event ? { ...event } : null);
    if (event || q.pending) {
      eventTimer.current = setTimeout(
        () => syncEvents(),
        event ? Math.max(1, q.deadline - performance.now()) : 30000,
      );
    }
  }, [isInteracting]);
  useEffect(() => {
    let disposed = false;
    const subscriptions: Promise<() => void>[] = [];
    api
      .loadSettings()
      .then((s) => {
        if (!disposed) {
          setSettings(s);
          setReady(true);
        }
      })
      .catch((e) => {
        if (!disposed) {
          report(e);
          setReady(true);
        }
      });
    const update = (value: Snapshot | null) => {
      if (!disposed) setSnapshot(value);
    };
    // Subscribe before reading the cache so an event cannot be lost during startup.
    subscriptions.push(
      api.subscribe<Snapshot>("orbit-snapshot", update).then((off) => {
        void api.getSnapshot().then(update).catch(report);
        return off;
      }),
    );
    subscriptions.push(
      api.subscribe<OrbitEvent>("orbit-event", (event) => {
        if (
          disposed ||
          !visibleRef.current ||
          event.kind === "NOTE_REMINDER_DUE"
        )
          return;
        const s = settingsRef.current;
        if (
          (event.kind === "VOLUME_CHANGED" &&
            (!s.volumeNotifications ||
              viewRef.current !== "closed" ||
              isInteracting())) ||
          (event.kind.startsWith("NETWORK_") && !s.networkNotifications) ||
          ((event.kind.startsWith("BATTERY_") ||
            event.kind.startsWith("CHARGER_")) &&
            !s.chargingNotifications)
        )
          return;
        manager.current.push(event, performance.now(), isInteracting());
        syncEvents();
      }),
    );
    subscriptions.push(
      api.subscribe<boolean>("orbit-visible", (show) => {
        if (disposed) return;
        setVisible(show);
        if (!show) {
          manager.current.clear();
          setNotification(null);
          setView("closed");
          clearTimeout(eventTimer.current);
          interacting.current = false;
        }
      }),
    );
    subscriptions.push(
      api.subscribe("measure-current", () => profileRef.current()),
    );
    subscriptions.push(
      api.onSettings(() => {
        manager.current.clear();
        setNotification(null);
        setVisible(true);
        setView("settings");
      }),
    );
    return () => {
      disposed = true;
      for (const p of subscriptions) void p.then((off) => off()).catch(report);
    };
  }, [report, syncEvents, isInteracting]);
  useEffect(() => {
    if (!ready) return;
    const active =
      visible &&
      (view === "system" || view === "ambient") &&
      settings.systemEnabled &&
      settings.compactMetrics &&
      !notification;
    void api.monitoring(active).catch(report);
    return () => {
      void api.monitoring(false).catch(report);
    };
  }, [
    view,
    ready,
    visible,
    settings.systemEnabled,
    settings.compactMetrics,
    !!notification,
    report,
  ]);
  const count =
    settings.quickNotes && settings.paperStack ? paperCount(activeNotes) : 0;
  const [regionCount, setRegionCount] = useState(count);
  useEffect(() => {
    if (count >= regionCount) {
      setRegionCount(count);
      return;
    }
    const t = setTimeout(
      () => setRegionCount(count),
      settings.noteAnimations ? 180 : 0,
    );
    return () => clearTimeout(t);
  }, [count, regionCount, settings.noteAnimations]);
  const ambientHeight = hasMedia
    ? hasNotes && settings.quickNotes
      ? 510
      : 400
    : hasNotes && settings.quickNotes
      ? 370
      : 260;
  const panelNotice =
    notification?.kind === "VOLUME_CHANGED" ? null : notification;
  const volumeHud =
    view === "closed" && notification?.kind === "VOLUME_CHANGED";
  const currentHeight =
    view === "ambient"
      ? (measuredHeight ?? ambientHeight)
      : panelHeight(view, !!panelNotice);
  useEffect(() => {
    if (!ready || !visible) return;
    const expanded = view !== "closed" || !!panelNotice;
    const delay = expanded || volumeHud ? 0 : 500 / settings.animationSpeed;
    const t = setTimeout(() => {
      resizeChain.current = resizeChain.current
        .catch(() => {})
        .then(() =>
          api.resize(
            expanded,
            settings,
            panelNotice ? 220 : currentHeight,
            volumeHud ? 0 : regionCount,
          ),
        )
        .catch(report);
    }, delay);
    return () => clearTimeout(t);
  }, [
    view,
    settings,
    ready,
    visible,
    panelNotice,
    volumeHud,
    report,
    currentHeight,
    regionCount,
  ]);
  useEffect(() => {
    clearTimeout(timer.current);
    clearTimeout(hoverTimer.current);
    syncEvents();
  }, [view, syncEvents]);
  const cancelExpand = () => clearTimeout(hoverTimer.current);
  const scheduleExpand = () => {
    cancelExpand();
    hoverTimer.current = setTimeout(() => setView("ambient"), 180);
  };
  const cancelCollapse = () => {
    clearTimeout(timer.current);
    pointerRef.current = true;
    interacting.current = true;
    if (manager.current.current?.kind === "VOLUME_CHANGED") {
      manager.current.dismiss(performance.now(), true);
      syncEvents();
    }
  };
  // One collapse timer, shared by pointer exit and interaction completion.
  const armCollapse = useCallback(() => {
    clearTimeout(timer.current);
    if (
      viewRef.current === "settings" ||
      !canCollapse(
        focusRef.current || profiling.current || mediaInteraction.current,
        dirtyRef.current,
        pointerRef.current,
      )
    )
      return;
    timer.current = setTimeout(() => {
      if (
        viewRef.current !== "settings" &&
        canCollapse(
          focusRef.current || profiling.current || mediaInteraction.current,
          dirtyRef.current,
          pointerRef.current,
        )
      )
        setView("closed");
    }, settingsRef.current.collapseDelay * 1000);
  }, []);
  const scheduleCollapse = () => {
    pointerRef.current = false;
    interacting.current = false;
    syncEvents();
    armCollapse();
  };
  useEffect(
    () => () => {
      clearTimeout(timer.current);
      clearTimeout(hoverTimer.current);
      clearTimeout(eventTimer.current);
    },
    [],
  );
  const setEditing = useCallback(
    (value: boolean) => {
      dirtyRef.current = value;
      armCollapse();
    },
    [armCollapse],
  );
  const setMediaInteraction = useCallback(
    (value: boolean) => {
      mediaInteraction.current = value;
      armCollapse();
    },
    [armCollapse],
  );
  const setFocused = useCallback(
    (value: boolean) => {
      focusRef.current = value;
      clearTimeout(timer.current);
      if (!value && !pointerRef.current) {
        syncEvents();
        armCollapse();
      }
    },
    [armCollapse, syncEvents],
  );
  useEffect(() => {
    const blur = () => {
      focusRef.current = false;
      pointerRef.current = false;
      scheduleCollapse();
    };
    window.addEventListener("blur", blur);
    return () => window.removeEventListener("blur", blur);
  }, []);
  const navigate = (next: View) => {
    if (next === "closed" && dirtyRef.current) return;
    manager.current.dismiss(performance.now(), true);
    syncEvents();
    setView(next);
  };
  const dismiss = () => {
    manager.current.dismiss(performance.now(), isInteracting());
    syncEvents();
  };
  const change = (s: Settings) => {
    setSettings(s);
    settingsRef.current = s;
    manager.current.clear();
    setNotification(null);
    saveChain.current = saveChain.current
      .catch(() => {})
      .then(() => api.saveSettings(s))
      .then(() => setError(""))
      .catch(report);
    return saveChain.current;
  };
  const measure = async () => {
    if (dirtyRef.current) {
      setPerformanceResult("Save or cancel the note before measuring.");
      return;
    }
    setPerformanceResult("Sampling ORBIT + WebView2 for 20 seconds…");
    manager.current.clear();
    setNotification(null);
    setView("closed");
    try {
      const raw = await api.measurePerformance();
      const result = JSON.parse(raw);
      setPerformanceResult(
        result.cpuPercent.toFixed(2) +
          "% CPU · " +
          result.workingSetMib.toFixed(1) +
          " MiB working set · " +
          result.privateMib.toFixed(1) +
          " MiB private · " +
          result.measuredProcesses +
          "/" +
          result.discoveredProcesses +
          " processes",
      );
    } catch (e) {
      setPerformanceResult(String(e));
    }
    setView("settings");
  };
  profileRef.current = async () => {
    if (profiling.current) return;
    profiling.current = true;
    clearTimeout(timer.current);
    setPerformanceResult("Sampling current view + WebView2 for 20 seconds…");
    try {
      const result = JSON.parse(await api.measurePerformance("current-view"));
      setPerformanceResult(
        result.cpuPercent.toFixed(2) +
          "% CPU · " +
          result.workingSetMib.toFixed(1) +
          " MiB working set · " +
          result.privateMib.toFixed(1) +
          " MiB private",
      );
    } catch (e) {
      setPerformanceResult(String(e));
    } finally {
      profiling.current = false;
      scheduleCollapse();
    }
  };
  return {
    count,
    currentHeight,
    setEditing,
    setMediaInteraction,
    setFocused,
    visible,
    performanceResult,
    measure,
    settings,
    snapshot,
    battery: (snapshot?.battery ?? null) as Battery | null,
    view,
    setView: navigate,
    error,
    report,
    change,
    notification,
    dismiss,
    cancelCollapse,
    scheduleCollapse,
    cancelExpand,
    scheduleExpand,
  };
}

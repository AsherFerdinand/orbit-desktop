import { afterEach, expect, it, vi } from "vitest";
import { EventManager, type OrbitEvent } from "../src/services/events";

// Run the hook's subscription effect without a DOM. State rerenders are unnecessary
// here: the interaction handlers and native event callback read mutable refs.
const harness = vi.hoisted(() => ({
  effects: [] as Array<() => void | (() => void)>,
  listeners: new Map<string, (event: any) => void>(),
  settings: false,
  view: "closed",
  viewWrites: [] as unknown[],
}));
vi.mock("react", () => ({
  useRef: (value: unknown) => ({ current: value }),
  useState: (value: unknown) => [
    value === "closed" ? (harness.settings ? "settings" : harness.view) : value,
    value === "closed"
      ? (next: unknown) => harness.viewWrites.push(next)
      : vi.fn(),
  ],
  useCallback: (fn: unknown) => fn,
  useEffect: (fn: () => void) => harness.effects.push(fn),
}));
vi.mock("../src/services/native", async () => {
  const { defaults } = await import("../src/types");
  return {
    loadSettings: () => Promise.resolve(defaults),
    getSnapshot: () => Promise.resolve(null),
    subscribe: (name: string, fn: (event: unknown) => void) => {
      harness.listeners.set(name, fn);
      return Promise.resolve(() => harness.listeners.delete(name));
    },
    onSettings: () => Promise.resolve(() => {}),
  };
});
import { useOrbit } from "../src/hooks/useOrbit";

afterEach(() => {
  vi.clearAllTimers();
  vi.useRealTimers();
  vi.restoreAllMocks();
  harness.view = "closed";
  harness.settings = false;
  harness.viewWrites = [];
});
it("defers incoming and dismissed notices for focus, editing, pointer and settings while preserving urgent priority", async () => {
  vi.useFakeTimers();
  const push = vi.spyOn(EventManager.prototype, "push");
  const notice = (kind: OrbitEvent["kind"], priority: number): OrbitEvent => ({
    kind,
    priority,
    title: kind,
    detail: "fixture",
    durationMs: 1000,
  });
  for (const mode of ["focus", "editing", "pointer", "settings"]) {
    harness.effects = [];
    harness.listeners.clear();
    harness.settings = mode === "settings";
    const orbit = useOrbit();
    const dispose = harness.effects[0]();
    if (mode === "focus") orbit.setFocused(true);
    if (mode === "editing") orbit.setEditing(true);
    if (mode === "pointer") orbit.cancelCollapse();
    const emit = harness.listeners.get("orbit-event")!;
    emit(notice("NETWORK_DISCONNECTED", 75));
    const queue = push.mock.contexts.at(-1) as EventManager;
    expect(queue.current, mode).toBeNull();
    expect(queue.pending, mode).toBe(1);
    emit(notice("BATTERY_CRITICAL", 100));
    expect(queue.current?.kind, mode).toBe("BATTERY_CRITICAL");
    orbit.dismiss();
    expect(queue.current, mode).toBeNull();
    expect(queue.pending, mode).toBe(1);
    if (mode !== "settings") {
      orbit.setEditing(false);
      orbit.setFocused(false);
      orbit.scheduleCollapse();
      expect(queue.current?.kind, mode).toBe("NETWORK_DISCONNECTED");
    }
    harness.listeners.get("orbit-visible")!(false);
    if (typeof dispose === "function") dispose();
    await Promise.resolve();
    vi.clearAllTimers();
  }
});

it("cancels collapse on re-entry and resumes the 400ms delay after editing or focus ends", () => {
  vi.useFakeTimers();
  harness.view = "ambient";
  harness.effects = [];
  const orbit = useOrbit();
  orbit.scheduleCollapse();
  vi.advanceTimersByTime(200);
  orbit.cancelCollapse();
  vi.advanceTimersByTime(1000);
  expect(harness.viewWrites).toEqual([]);
  orbit.scheduleCollapse();
  vi.advanceTimersByTime(399);
  expect(harness.viewWrites).toEqual([]);
  vi.advanceTimersByTime(1);
  expect(harness.viewWrites).toEqual(["closed"]);
  harness.viewWrites = [];
  orbit.setMediaInteraction(true);
  orbit.setFocused(false);
  orbit.scheduleCollapse();
  vi.advanceTimersByTime(1000);
  expect(harness.viewWrites).toEqual([]);
  orbit.setMediaInteraction(false);
  vi.advanceTimersByTime(399);
  expect(harness.viewWrites).toEqual([]);
  vi.advanceTimersByTime(1);
  expect(harness.viewWrites).toEqual(["closed"]);
  harness.viewWrites = [];
  orbit.setEditing(true);
  orbit.scheduleCollapse();
  vi.advanceTimersByTime(1000);
  expect(harness.viewWrites).toEqual([]);
  orbit.setFocused(true);
  orbit.setEditing(false);
  vi.advanceTimersByTime(1000);
  expect(harness.viewWrites).toEqual([]);
  orbit.setFocused(false);
  vi.advanceTimersByTime(399);
  expect(harness.viewWrites).toEqual([]);
  vi.advanceTimersByTime(1);
  expect(harness.viewWrites).toEqual(["closed"]);
});
it("ignores volume notices during expanded or active interaction while allowing a collapsed HUD", async () => {
  vi.useFakeTimers();
  const push = vi.spyOn(EventManager.prototype, "push");
  for (const mode of ["expanded", "editing", "collapsed"]) {
    harness.view = mode === "expanded" ? "ambient" : "closed";
    harness.effects = [];
    harness.listeners.clear();
    const orbit = useOrbit();
    const dispose = harness.effects[0]();
    if (mode === "editing") orbit.setEditing(true);
    push.mockClear();
    harness.listeners.get("orbit-event")!({
      kind: "NOTE_REMINDER_DUE",
      title: "Note reminder",
      detail: "Fixture",
      priority: 90,
      durationMs: 4000,
    });
    expect(push).not.toHaveBeenCalled();
    harness.listeners.get("orbit-event")!({
      kind: "VOLUME_CHANGED",
      title: "System volume",
      detail: "38%",
      priority: 30,
      durationMs: 4000,
    });
    expect(push).toHaveBeenCalledTimes(mode === "collapsed" ? 1 : 0);
    if (mode === "collapsed") {
      const queue = push.mock.contexts[0] as EventManager;
      expect(queue.current?.detail).toBe("38%");
      orbit.cancelCollapse();
      expect(queue.current).toBeNull();
    }
    harness.listeners.get("orbit-visible")!(false);
    if (typeof dispose === "function") dispose();
    await Promise.resolve();
    vi.clearAllTimers();
  }
});

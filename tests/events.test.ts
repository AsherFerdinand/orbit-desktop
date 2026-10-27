import { describe, it, expect } from "vitest";
import {
  EventManager,
  panelHeight,
  type OrbitEvent,
} from "../src/services/events";
const event = (
  kind: OrbitEvent["kind"],
  priority = 30,
  durationMs = 1000,
): OrbitEvent => ({
  kind,
  title: kind,
  detail: "real event payload in production",
  priority,
  durationMs,
});
describe("notification scheduler", () => {
  it("deduplicates repeated transitions across display and cooldown", () => {
    const q = new EventManager();
    q.push(event("CHARGER_CONNECTED", 80), 0);
    q.push(event("CHARGER_CONNECTED", 80), 10);
    expect(q.pending).toBe(0);
    q.tick(1001);
    expect(q.current).toBeNull();
    q.push(event("CHARGER_CONNECTED", 80), 2000);
    expect(q.current).toBeNull();
    q.push(event("CHARGER_CONNECTED", 80), 30001);
    expect(q.current?.kind).toBe("CHARGER_CONNECTED");
  });
  it("critical battery preempts information and prevents lower-priority interruption", () => {
    const q = new EventManager();
    q.push(event("VOLUME_CHANGED"), 0);
    q.push(event("BATTERY_CRITICAL", 100), 10);
    q.push(event("NETWORK_RECONNECTED", 40), 20);
    expect(q.current?.kind).toBe("BATTERY_CRITICAL");
    expect(q.pending).toBe(1);
    expect(q.tick(1011)?.kind).toBe("NETWORK_RECONNECTED");
  });
  it("defers informational events during interaction but permits battery warnings", () => {
    const q = new EventManager();
    expect(q.push(event("NETWORK_DISCONNECTED", 75), 0, true)).toBeNull();
    expect(q.push(event("BATTERY_LOW", 85), 10, true)?.kind).toBe(
      "BATTERY_LOW",
    );
    q.tick(1011, true);
    expect(q.current).toBeNull();
    expect(q.tick(1012, false)?.kind).toBe("NETWORK_DISCONNECTED");
  });
  it("coalesces volume changes and expires one second after the most recent update", () => {
    const q = new EventManager();
    q.push({ ...event("VOLUME_CHANGED"), detail: "10%" }, 0, true);
    q.push({ ...event("VOLUME_CHANGED"), detail: "20%" }, 10, true);
    expect(q.pending).toBe(1);
    expect(q.tick(11, false)?.detail).toBe("20%");
    q.push({ ...event("VOLUME_CHANGED", 30, 4000), detail: "30%" }, 20);
    expect(q.current?.detail).toBe("30%");
    expect(q.deadline).toBe(1020);
    expect(q.tick(1019)?.detail).toBe("30%");
    expect(q.tick(1020)).toBeNull();
  });
  it("expires stale queued notices", () => {
    const q = new EventManager();
    q.push(event("VOLUME_CHANGED"), 0, true);
    expect(q.tick(1001)).toBeNull();
    expect(q.pending).toBe(0);
  });
  it("preserves FIFO order at equal priority", () => {
    const q = new EventManager();
    q.push(event("CHARGER_CONNECTED", 80), 0, true);
    q.push(event("CHARGER_DISCONNECTED", 80), 10, true);
    expect(q.tick(11)?.kind).toBe("CHARGER_CONNECTED");
    expect(q.tick(1012)?.kind).toBe("CHARGER_DISCONNECTED");
  });
  it("clears pending and active events when hidden", () => {
    const q = new EventManager();
    q.push(event("BATTERY_CRITICAL", 100), 0);
    q.push(event("NETWORK_DISCONNECTED", 75), 10);
    q.clear();
    expect(q.tick(20)).toBeNull();
    expect(q.pending).toBe(0);
  });
  it("dismisses and progresses predictably during a burst", () => {
    const q = new EventManager();
    q.push(event("BATTERY_LOW", 85), 0);
    q.push(event("CHARGER_CONNECTED", 80), 1);
    expect(q.dismiss(10)?.kind).toBe("CHARGER_CONNECTED");
    expect(q.tick(1011)).toBeNull();
  });
});
describe("panel geometry decisions", () => {
  it("uses compact notice bounds and restores selected panel size", () => {
    expect(panelHeight("settings", true)).toBe(220);
    expect(panelHeight("settings", false)).toBe(490);
    expect(panelHeight("system", false)).toBe(470);
    expect(panelHeight("volume", false)).toBe(370);
  });
});

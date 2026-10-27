export type EventKind =
  | "NOTE_REMINDER_DUE"
  | "BATTERY_LOW"
  | "BATTERY_CRITICAL"
  | "CHARGER_CONNECTED"
  | "CHARGER_DISCONNECTED"
  | "BATTERY_FULL"
  | "VOLUME_CHANGED"
  | "NETWORK_DISCONNECTED"
  | "NETWORK_RECONNECTED";
export interface OrbitEvent {
  kind: EventKind;
  title: string;
  detail: string;
  priority: number;
  durationMs: number;
}
interface Pending {
  event: OrbitEvent;
  expires: number;
  sequence: number;
}
// Rendering-independent, deterministic scheduler. The caller supplies a monotonic clock.
export class EventManager {
  current: OrbitEvent | null = null;
  deadline = 0;
  private queue: Pending[] = [];
  private seen = new Map<EventKind, number>();
  private sequence = 0;
  private advance(now: number, interacting: boolean) {
    this.queue = this.queue.filter((p) => p.expires > now);
    if (this.current && now >= this.deadline) this.current = null;
    if (!this.current) {
      const index = this.queue.findIndex(
        (p) => !interacting || p.event.priority >= 85,
      );
      if (index >= 0) {
        this.current = this.queue.splice(index, 1)[0].event;
        this.deadline = now + this.current.durationMs;
      }
    }
    return this.current;
  }
  push(event: OrbitEvent, now: number, interacting = false) {
    if (event.kind === "VOLUME_CHANGED") event = { ...event, durationMs: 1000 };
    this.advance(now, interacting);
    const last = this.seen.get(event.kind);
    const cooldown = 30000;
    if (event.kind === "VOLUME_CHANGED") {
      if (this.current?.kind === event.kind) {
        this.current = event;
        this.deadline = now + event.durationMs;
        return this.current;
      }
      this.queue = this.queue.filter((p) => p.event.kind !== event.kind);
    } else if (
      (last !== undefined && now - last < cooldown) ||
      this.current?.kind === event.kind ||
      this.queue.some((p) => p.event.kind === event.kind)
    )
      return this.current;
    this.seen.set(event.kind, now);
    this.queue.push({
      event,
      expires: now + (event.kind === "VOLUME_CHANGED" ? 1000 : 30000),
      sequence: this.sequence++,
    });
    this.queue.sort(
      (a, b) => b.event.priority - a.event.priority || a.sequence - b.sequence,
    );
    this.queue = this.queue.slice(0, 16);
    if (
      this.current &&
      event.priority > this.current.priority &&
      event.priority >= 85
    ) {
      this.current = null;
    } // Higher urgency wins; the displaced informational event is intentionally discarded.
    return this.advance(now, interacting);
  }
  tick(now: number, interacting = false) {
    return this.advance(now, interacting);
  }
  dismiss(now: number, interacting = false) {
    this.current = null;
    return this.advance(now, interacting);
  }
  clear() {
    this.current = null;
    this.queue = [];
    this.deadline = 0;
  }
  get pending() {
    return this.queue.length;
  }
}
export type View =
  "closed" | "ambient" | "battery" | "system" | "volume" | "media" | "settings";
export function panelHeight(view: View, notification: boolean) {
  return notification
    ? 220
    : view === "settings"
      ? 490
      : view === "media"
        ? 470
        : view === "system"
          ? 470
          : view === "volume"
            ? 370
            : 410;
}

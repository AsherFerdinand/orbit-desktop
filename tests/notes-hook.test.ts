import { expect, it, vi } from "vitest";
const fixture = vi.hoisted(() => ({
  states: 0,
  effects: [] as Array<() => void | (() => void)>,
  notes: [] as unknown[],
  changed: (() => {}) as () => void,
  calls: [] as Array<[string, any]>,
  fired: false,
}));
vi.mock("react", () => ({
  useState: (value: unknown) => {
    const index = fixture.states++;
    return [
      index === 2 ? true : value,
      (next: unknown) => {
        if (index === 0) fixture.notes = next as unknown[];
      },
    ];
  },
  useRef: (value: unknown) => ({ current: value }),
  useCallback: (fn: unknown) => fn,
  useEffect: (fn: () => void) => fixture.effects.push(fn),
}));
vi.mock("../src/services/native", () => ({
  native: true,
  subscribe: (_name: string, handler: () => void) => {
    fixture.changed = handler;
    return Promise.resolve(() => {});
  },
}));
vi.mock("@tauri-apps/api/core", () => ({
  invoke: async (command: string, args: any) => {
    fixture.calls.push([command, args]);
    if (command === "mutate_note") fixture.changed();
    return [
      {
        id: "n",
        reminderAt: 123,
        reminderEnabled: !fixture.fired,
        reminderFired: fixture.fired,
      },
    ];
  },
}));
import { useNotes } from "../src/hooks/useNotes";
it("serializes reminder mutations with scheduler refreshes and reloads fired state", async () => {
  const store = useNotes();
  const cleanup = fixture.effects[0]();
  const flush = async () => {
    for (let i = 0; i < 20; i++) await Promise.resolve();
  };
  await flush();
  const reminder = { at: 123, enabled: true };
  expect(await store.mutate("create", null, "Fixture", null, reminder)).toBe(
    true,
  );
  await flush();
  expect(
    fixture.calls.find(([command]) => command === "mutate_note")?.[1].reminder,
  ).toEqual(reminder);
  fixture.fired = true;
  fixture.changed();
  await flush();
  expect(fixture.notes).toEqual([
    { id: "n", reminderAt: 123, reminderEnabled: false, reminderFired: true },
  ]);
  expect(fixture.calls.map(([command]) => command)).toEqual([
    "list_notes",
    "mutate_note",
    "list_notes",
    "list_notes",
  ]);
  if (typeof cleanup === "function") cleanup();
});

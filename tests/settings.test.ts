import { afterEach, beforeEach, expect, it, vi } from "vitest";
const harness = vi.hoisted(() => ({
  values: [] as any[],
  effects: [] as Array<() => void | (() => void)>,
  stateIndex: 0,
  listeners: new Map<string, () => void>(),
  get: vi.fn(),
  set: vi.fn(),
}));
vi.mock("react", async (importOriginal) => {
  const actual = await importOriginal<typeof import("react")>();
  return {
    ...actual,
    useState: (value: unknown) => {
      const index = harness.stateIndex++;
      harness.values[index] = value;
      return [
        value,
        (next: unknown) => {
          harness.values[index] = next;
        },
      ];
    },
    useRef: (value: unknown) => ({ current: value }),
    useCallback: (fn: unknown) => fn,
    useEffect: (fn: () => void) => harness.effects.push(fn),
  };
});
vi.mock("../src/services/native", () => ({
  native: true,
  getWindowsTheme: () => harness.get(),
  setWindowsTheme: (mode: string) => harness.set(mode),
}));
import { WindowsAppearance } from "../src/modules/Settings";
let cleanup: void | (() => void);
const flush = async () => {
  for (let i = 0; i < 10; i++) await Promise.resolve();
};
beforeEach(() => {
  harness.values = [];
  harness.effects = [];
  harness.stateIndex = 0;
  harness.listeners.clear();
  harness.get.mockReset();
  harness.set.mockReset();
  const events = {
    addEventListener: (name: string, fn: () => void) =>
      harness.listeners.set(name, fn),
    removeEventListener: (name: string) => harness.listeners.delete(name),
  };
  vi.stubGlobal("window", events);
  vi.stubGlobal("document", { ...events, visibilityState: "visible" });
});
afterEach(() => {
  if (cleanup) cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});
function mount() {
  const tree = WindowsAppearance() as any;
  cleanup = harness.effects[0]();
  return tree.props.children[0].props.children[1].props.onChange as (
    event: any,
  ) => void;
}
it("reads actual state on open/focus and prevents stale reads from overwriting a user change", async () => {
  harness.get.mockResolvedValueOnce("dark").mockResolvedValueOnce("mixed");
  const change = mount();
  await flush();
  expect(harness.values[0]).toBe("dark");
  expect(harness.set).not.toHaveBeenCalled();
  harness.listeners.get("focus")!();
  await flush();
  expect(harness.values[0]).toBe("mixed");
  let oldRead!: (mode: string) => void;
  harness.get.mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        oldRead = resolve;
      }),
  );
  harness.listeners.get("focus")!();
  let changed!: (value: unknown) => void;
  harness.set.mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        changed = resolve;
      }),
  );
  change({ target: { value: "light" } });
  expect(harness.values[0]).toBe("mixed"); // No optimistic/fake Windows state.
  expect(harness.values[1]).toBe(true);
  expect(harness.set).toHaveBeenCalledExactlyOnceWith("light");
  changed({ mode: "light", warning: null });
  await flush();
  oldRead("dark");
  await flush();
  expect(harness.values[0]).toBe("light");
  expect(harness.values[1]).toBe(false);
});
it("represents read failure as unknown and re-reads actual state after a failed write", async () => {
  vi.spyOn(console, "warn").mockImplementation(() => {});
  harness.get
    .mockRejectedValueOnce(new Error("Read denied"))
    .mockResolvedValueOnce("dark");
  harness.set.mockRejectedValueOnce(new Error("Write denied"));
  const change = mount();
  await flush();
  expect(harness.values[0]).toBeNull();
  expect(harness.values[2]).toContain("Read denied");
  change({ target: { value: "light" } });
  await flush();
  expect(harness.values[0]).toBe("dark");
  expect(harness.values[2]).toContain("Write denied");
  expect(harness.values[1]).toBe(false);
  expect(harness.get).toHaveBeenCalledTimes(2);
});

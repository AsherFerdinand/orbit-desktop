import { expect, it, vi } from "vitest";
const harness = vi.hoisted(() => ({
  snapshot: (() => "") as () => string,
  updates: [] as string[],
  off: (() => {}) as () => void,
}));
vi.mock("react", () => ({
  useMemo: (fn: () => unknown) => fn(),
  useCallback: (fn: unknown) => fn,
  useSyncExternalStore: (
    subscribe: (fn: () => void) => () => void,
    snapshot: () => string,
  ) => {
    harness.snapshot = snapshot;
    harness.off = subscribe(() => harness.updates.push(snapshot()));
    return snapshot();
  },
}));
import { useSystemTheme } from "../src/hooks/useSystemTheme";
it("follows Windows preference events in System mode and removes the subscription", () => {
  let changed!: () => void;
  const query = {
    matches: false,
    addEventListener: vi.fn((_name, callback) => {
      changed = callback;
    }),
    removeEventListener: vi.fn(),
  };
  const matchMedia = vi.fn(() => query);
  vi.stubGlobal("window", { matchMedia });
  try {
    expect(useSystemTheme(true)).toBe("light");
    expect(matchMedia).toHaveBeenCalledExactlyOnceWith(
      "(prefers-color-scheme: dark)",
    );
    query.matches = true;
    changed();
    expect(harness.updates).toEqual(["dark"]);
    harness.off();
    expect(query.removeEventListener).toHaveBeenCalledWith("change", changed);
    useSystemTheme(false);
    expect(matchMedia).toHaveBeenCalledTimes(1);
    harness.off();
  } finally {
    vi.unstubAllGlobals();
  }
});

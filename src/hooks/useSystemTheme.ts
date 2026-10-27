import { useCallback, useMemo, useSyncExternalStore } from "react";
// WebView2's standard color-scheme preference follows Windows application mode.
// Subscribe only in System mode; React removes the listener on change/unmount.
export function useSystemTheme(enabled: boolean): "dark" | "light" {
  const query = useMemo(
    () =>
      enabled &&
      typeof window !== "undefined" &&
      typeof window.matchMedia === "function"
        ? window.matchMedia("(prefers-color-scheme: dark)")
        : null,
    [enabled],
  );
  const subscribe = useCallback(
    (changed: () => void) => {
      query?.addEventListener("change", changed);
      return () => query?.removeEventListener("change", changed);
    },
    [query],
  );
  return useSyncExternalStore(
    subscribe,
    () => (query?.matches ? "dark" : "light"),
    () => "dark",
  );
}

import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Settings, Snapshot } from "../types";
import { defaults } from "../types";
export const native = isTauri();
export const loadSettings = () =>
  native ? invoke<Settings>("load_settings") : Promise.resolve(defaults);
export const saveSettings = (settings: Settings) =>
  native ? invoke<void>("save_settings", { settings }) : Promise.resolve();
export const resize = (
  expanded: boolean,
  settings: Settings,
  panelHeight: number,
  paperCount = 0,
) =>
  native
    ? invoke<void>("resize_island", {
        expanded,
        width: settings.width,
        height: settings.height,
        panelHeight,
        paperCount,
      })
    : Promise.resolve();
export const hide = () =>
  native ? invoke<void>("hide_island") : Promise.resolve();
export const onSettings = (fn: () => void) =>
  native ? listen("open-settings", fn) : Promise.resolve(() => {});

export const getSnapshot = () =>
  native ? invoke<Snapshot>("get_snapshot") : Promise.resolve(null);
export const monitoring = (active: boolean) =>
  native ? invoke<void>("set_monitoring", { active }) : Promise.resolve();
export const subscribe = <T>(name: string, fn: (value: T) => void) =>
  native ? listen<T>(name, (e) => fn(e.payload)) : Promise.resolve(() => {});

export const measurePerformance = (context = "collapsed-idle") =>
  native
    ? invoke<string>("measure_performance", { context })
    : Promise.reject(new Error("Requires Windows desktop application"));

export const showSystemMenu = () =>
  native ? invoke<void>("show_system_menu") : Promise.resolve();

export type WindowsTheme = "light" | "dark" | "mixed";
export interface WindowsThemeChange {
  mode: WindowsTheme | null;
  warning: string | null;
}
export const getWindowsTheme = () =>
  native
    ? invoke<WindowsTheme | null>("get_windows_theme")
    : Promise.resolve(null);
export const setWindowsTheme = (mode: "light" | "dark") =>
  native
    ? invoke<WindowsThemeChange>("set_windows_theme", { mode })
    : Promise.reject(new Error("Windows appearance requires the desktop app"));

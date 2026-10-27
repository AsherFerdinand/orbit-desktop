import { useCallback, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { native } from "../services/native";
// Push actual animated geometry; no idle poll, no extra window or per-frame resize.
export function useIslandRegion() {
  const body = useRef<HTMLElement>(null),
    shell = useRef<HTMLDivElement>(null);
  const state = useRef({
    frame: 0,
    last: 0,
    pending: false,
    again: false,
    disposed: false,
    lastShape: "",
  });
  const update = useCallback(() => {
    const s = state.current;
    if (!native || s.disposed || s.frame) return;
    s.frame = requestAnimationFrame(async () => {
      s.frame = 0;
      if (s.disposed) return;
      if (s.pending) {
        s.again = true;
        return;
      }
      if (performance.now() - s.last < 32) {
        s.frame = window.setTimeout(() => {
          s.frame = 0;
          update();
        }, 32);
        return;
      }
      if (!body.current || !shell.current) return;
      const rect = (el: HTMLElement) => {
        const r = el.getBoundingClientRect();
        return {
          left: r.left,
          top: r.top,
          width: r.width,
          height: r.height,
          radius: parseFloat(getComputedStyle(el).borderTopLeftRadius) || 0,
        };
      };
      const payload = {
        body: rect(body.current),
        papers: Array.from(
          shell.current.querySelectorAll<HTMLElement>(".paper"),
        )
          .filter((el) => getComputedStyle(el).visibility !== "hidden")
          .slice(0, 3)
          .map(rect),
      };
      const key = JSON.stringify(payload);
      if (key === s.lastShape) return;
      s.pending = true;
      s.last = performance.now();
      try {
        await invoke("set_island_region", payload);
        s.lastShape = key;
      } catch (e) {
        console.warn("ORBIT visual region:", e);
      } finally {
        s.pending = false;
        if (s.again && !s.disposed) {
          s.again = false;
          update();
        }
      }
    });
  }, []);
  useEffect(() => {
    const s = state.current;
    s.disposed = false;
    window.addEventListener("resize", update);
    update();
    return () => {
      s.disposed = true;
      cancelAnimationFrame(s.frame);
      clearTimeout(s.frame);
      s.frame = 0;
      window.removeEventListener("resize", update);
    };
  }, [update]);
  return { body, shell, update };
}

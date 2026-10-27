import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { native, subscribe } from "../services/native";
export interface Note {
  id: string;
  text: string;
  createdAt: number;
  updatedAt: number;
  completed: boolean;
  reminderAt: number | null;
  reminderEnabled: boolean;
  reminderFired: boolean;
}
export function useNotes() {
  const [notes, setNotes] = useState<Note[]>([]),
    [error, setError] = useState(""),
    [ready, setReady] = useState(!native),
    [busy, setBusy] = useState(false);
  const chain = useRef(Promise.resolve());
  useEffect(() => {
    let disposed = false;
    let off = () => {};
    const refresh = () => {
      const task = chain.current
        .catch(() => {})
        .then(async () => {
          try {
            const value = await invoke<Note[]>("list_notes");
            if (!disposed) {
              setNotes(value);
              setReady(true);
              setError("");
            }
          } catch (e) {
            if (!disposed) setError(String(e));
          }
        })
        .finally(() => {
          if (!disposed && chain.current === task) setBusy(false);
        });
      chain.current = task;
    };
    if (native)
      void subscribe("orbit-notes", refresh)
        .then((unlisten) => {
          off = unlisten;
          if (disposed) {
            off();
            return;
          }
          refresh();
        })
        .catch((e) => {
          if (!disposed) setError(String(e));
        });
    return () => {
      disposed = true;
      off();
    };
  }, []);
  const mutate = useCallback(
    (
      action: string,
      id: string | null,
      text: string | null,
      completed: boolean | null,
      reminder?: { at: number | null; enabled: boolean },
    ) => {
      if (!native || !ready) return Promise.resolve(false);
      setBusy(true);
      let result = false;
      const task = chain.current
        .catch(() => {})
        .then(async () => {
          try {
            const value = await invoke<Note[]>("mutate_note", {
              action,
              id,
              text,
              completed,
              reminder: reminder ?? null,
            });
            setNotes(value);
            setError("");
            result = true;
          } catch (e) {
            setError(String(e));
          }
        });
      chain.current = task;
      return task
        .then(() => result)
        .finally(() => {
          if (chain.current === task) setBusy(false);
        });
    },
    [ready],
  );
  return {
    notes,
    error,
    ready,
    busy,
    mutate,
    active: notes.filter((n) => !n.completed).length,
  };
}

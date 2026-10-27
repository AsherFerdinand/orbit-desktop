import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { native, subscribe } from "../services/native";
import {
  chooseSession,
  artworkCache,
  ambientItem,
  canSeek,
  type MediaState,
  type MediaItem,
} from "../services/media";
const empty: MediaState = {
  revision: 0,
  sessions: [],
  currentId: null,
  error: null,
};
export function useMedia() {
  const [data, setData] = useState(empty),
    [selected, select] = useState<number | null>(null),
    [error, setError] = useState(""),
    [busy, setBusy] = useState(false);
  const [display, setDisplay] = useState<MediaItem | null>(null);
  useEffect(() => {
    let disposed = false;
    let off = () => {};
    let fetching = false,
      again = false;
    const update = (next: MediaState) => {
      if (!disposed)
        setData((old) => {
          if (next.revision < old.revision) return old;
          return next;
        });
    };
    const refresh = async () => {
      if (disposed) return;
      if (fetching) {
        again = true;
        return;
      }
      fetching = true;
      try {
        do {
          again = false;
          update(await invoke<MediaState>("get_media"));
        } while (again && !disposed);
      } catch (e) {
        if (!disposed) setError(String(e));
      } finally {
        fetching = false;
      }
    };
    if (native)
      void subscribe<number>("orbit-media", () => void refresh())
        .then((unlisten) => {
          off = unlisten;
          if (disposed) {
            off();
            return;
          }
          void refresh();
        })
        .catch((e) => !disposed && setError(String(e)));
    return () => {
      disposed = true;
      off();
    };
  }, []);
  const raw = chooseSession(data, selected);
  const [cover, setCover] = useState<{
    id: number;
    data: string | null;
  } | null>(null);
  const [loadCover] = useState(() =>
    artworkCache((id) => invoke<string | null>("get_media_artwork", { id })),
  );
  const artworkId = raw?.artworkId;
  useEffect(() => {
    let disposed = false;
    if (artworkId != null)
      void loadCover(artworkId).then((data) => {
        if (!disposed) setCover({ id: artworkId, data });
      });
    return () => {
      disposed = true;
    };
  }, [artworkId, loadCover]);
  const item = useMemo(
    () =>
      raw
        ? {
            ...raw,
            artwork:
              artworkId != null
                ? cover?.id === artworkId
                  ? cover.data
                  : null
                : null,
          }
        : undefined,
    [raw, artworkId, cover],
  );
  useEffect(() => {
    if (
      data.revision > 0 &&
      selected !== null &&
      !data.sessions.some((s) => s.id === selected)
    )
      select(null);
  }, [selected, data]);
  const live = ambientItem(item);
  useEffect(() => {
    if (live) setDisplay(live);
  }, [item]);
  const hasLive = !!live;
  useEffect(() => {
    if (hasLive) return;
    const timer = setTimeout(() => setDisplay(null), 2000);
    return () => clearTimeout(timer);
  }, [hasLive]);
  const valid =
    !!item &&
    !!display &&
    item.id === display.id &&
    (item.status === 4 || item.status === 5);
  async function act(action: string) {
    if (!valid || busy || !display) return;
    setBusy(true);
    setError("");
    try {
      await invoke("media_action", { id: display.id, action });
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  async function seek(position: number) {
    if (!valid || busy || !canSeek(display)) return;
    setBusy(true);
    try {
      const next = await invoke<MediaState>("media_seek", {
        id: display.id,
        position,
      });
      setData((old) => (next.revision >= old.revision ? next : old));
    } catch (e) {
      // Unsupported/declined seeks stay local diagnostics, not island notices.
      console.warn("ORBIT media seek:", e);
      try {
        const next = await invoke<MediaState>("get_media");
        setData((old) => (next.revision >= old.revision ? next : old));
      } catch (e) {
        console.warn("ORBIT media refresh:", e);
      }
    } finally {
      setBusy(false);
    }
  }
  return {
    data,
    selected,
    select,
    item: display,
    valid,
    busy,
    error: error || data.error,
    act,
    seek,
  };
}

export interface MediaItem {
  id: number;
  source: string;
  title: string;
  artist: string;
  subtitle: string;
  album: string;
  artwork: string | null;
  artworkId?: number | null;
  status: number;
  play: boolean;
  pause: boolean;
  toggle: boolean;
  seekable: boolean;
  next: boolean;
  previous: boolean;
  sampledAt: number;
  position: number | null;
  duration: number | null;
  rate: number;
  error: string | null;
}
// Native snapshots omit image bytes; only the displayed item is hydrated.
export type MediaSession = Omit<MediaItem, "artwork">;
export interface MediaState {
  revision: number;
  sessions: MediaSession[];
  currentId: number | null;
  error: string | null;
}
export function chooseSession(
  state: MediaState,
  selected: number | null,
): MediaSession | undefined {
  const s = state.sessions;
  return (
    s.find((x) => x.id === selected) ??
    s.find((x) => /brave/i.test(x.source) && x.status === 4) ??
    s.find((x) => /brave/i.test(x.source)) ??
    s.find((x) => x.id === state.currentId) ??
    s.find((x) => x.status === 4) ??
    s[0]
  );
}
export function progressAt(item: MediaItem, elapsed: number): number | null {
  if (
    item.position === null ||
    item.duration === null ||
    !Number.isFinite(item.position) ||
    !Number.isFinite(item.duration) ||
    item.duration <= 0
  )
    return null;
  const rate = Number.isFinite(item.rate) ? item.rate : 1;
  return Math.min(
    item.duration,
    Math.max(
      0,
      item.position + (item.status === 4 ? Math.max(0, elapsed) * rate : 0),
    ),
  );
}

export function reliableTimeline(
  item: MediaItem | null | undefined,
): item is MediaItem {
  return (
    !!item &&
    item.position !== null &&
    item.duration !== null &&
    Number.isFinite(item.position) &&
    Number.isFinite(item.duration) &&
    item.duration > 0 &&
    item.duration < 604800
  );
}
export function ambientItem(item: MediaItem | undefined) {
  return item && (item.status === 4 || item.status === 5) ? item : null;
}
export function paperCount(active: number) {
  return Math.min(3, Math.max(0, Math.floor(active)));
}
export function canCollapse(
  focused: boolean,
  dirty: boolean,
  pointer: boolean,
) {
  return !focused && !dirty && !pointer;
}

// Seconds relative to the start of the reported timeline, not WinRT ticks.
export function seekPosition(
  clientX: number,
  left: number,
  width: number,
  duration: number,
): number | null {
  if (
    ![clientX, left, width, duration].every(Number.isFinite) ||
    width <= 0 ||
    duration <= 0
  )
    return null;
  return Math.max(0, Math.min(1, (clientX - left) / width)) * duration;
}
export function canSeek(item: MediaItem | null | undefined): item is MediaItem {
  return (
    reliableTimeline(item) &&
    item.seekable &&
    (item.status === 4 || item.status === 5)
  );
}

// Four recent covers (including failed requests), scoped to the mounted media hook.
// Coalesce simultaneous loads; failures retry only after eviction / a genuine ID change.
export function artworkCache(fetch: (id: number) => Promise<string | null>) {
  const entries = new Map<number, Promise<string | null>>();
  return (id: number) => {
    let value = entries.get(id);
    if (!value) {
      value = fetch(id).catch(() => null);
      entries.set(id, value);
      if (entries.size > 4) entries.delete(entries.keys().next().value!);
    }
    return value;
  };
}

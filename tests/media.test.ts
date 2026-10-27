import { describe, it, expect } from "vitest";
import {
  chooseSession,
  progressAt,
  reliableTimeline,
  canSeek,
  seekPosition,
  type MediaItem,
  type MediaState,
} from "../src/services/media";
const item = (id: number, source = "Brave", status = 5): MediaItem => ({
  id,
  source,
  status,
  title: "",
  artist: "",
  subtitle: "",
  album: "",
  artwork: null,
  play: true,
  pause: true,
  toggle: false,
  seekable: true,
  next: false,
  previous: false,
  position: 20,
  duration: 100,
  rate: 1,
  sampledAt: 1000,
  error: null,
});
const state = (
  sessions: MediaItem[],
  currentId: number | null = null,
): MediaState => ({ sessions, currentId, revision: 1, error: null });
describe("Windows media session selection", () => {
  it("prefers playing Brave over paused Brave and Windows current non-Brave", () => {
    expect(
      chooseSession(
        state([item(1), item(2, "Chrome", 4), item(3, "Brave", 4)], 2),
        null,
      )?.id,
    ).toBe(3);
  });
  it("respects an explicit paused session even while another plays", () => {
    expect(chooseSession(state([item(1), item(2, "Brave", 4)]), 1)?.id).toBe(1);
  });
  it("keeps distinct session IDs with the same application ID", () => {
    const s = state([item(1), item(2)]);
    expect(chooseSession(s, 2)?.id).toBe(2);
    expect(chooseSession(s, 1)?.id).toBe(1);
  });
  it("falls back when a pinned session closes and handles no sessions", () => {
    expect(chooseSession(state([item(2)]), 1)?.id).toBe(2);
    expect(chooseSession(state([]), 1)).toBeUndefined();
  });
  it("uses Windows current when Brave is absent", () => {
    expect(
      chooseSession(state([item(1, "Other"), item(2, "Chrome")], 2), null)?.id,
    ).toBe(2);
  });
  it("tolerates missing source and metadata without inventing fields", () => {
    const s = item(1, "");
    expect(chooseSession(state([s]), null)?.title).toBe("");
  });
});
describe("reported media timeline", () => {
  it("advances playing sessions at supplied speed and clamps to duration", () => {
    const s = { ...item(1, "Brave", 4), rate: 2 };
    expect(progressAt(s, 3)).toBe(26);
    expect(progressAt(s, 999)).toBe(100);
  });
  it("freezes paused sessions and ignores negative elapsed time", () => {
    expect(progressAt(item(1), 60)).toBe(20);
    expect(progressAt(item(1, "Brave", 4), -10)).toBe(20);
  });
  it("does not fabricate unavailable or invalid timelines", () => {
    for (const s of [
      { ...item(1), duration: null },
      { ...item(1), position: null },
      { ...item(1), duration: 0 },
      { ...item(1), position: NaN },
    ])
      expect(progressAt(s, 10)).toBeNull();
  });
  it("bounds out-of-range positions", () => {
    expect(progressAt({ ...item(1), position: -5 }, 0)).toBe(0);
    expect(progressAt({ ...item(1), position: 150 }, 0)).toBe(100);
  });
});

describe("seek policy", () => {
  it("maps pointer fractions to seconds and clamps outside the line", () => {
    expect([10, 35, 60, 110].map((x) => seekPosition(x, 10, 100, 300))).toEqual(
      [0, 75, 150, 300],
    );
    expect(seekPosition(-20, 10, 100, 300)).toBe(0);
    expect(seekPosition(200, 10, 100, 300)).toBe(300);
    for (const [x, width, duration] of [
      [NaN, 100, 300],
      [0, 0, 300],
      [0, 100, 0],
      [0, 100, Infinity],
    ]) {
      expect(seekPosition(x, 0, width, duration)).toBeNull();
    }
  });
  it("keeps valid unsupported timelines read-only and rejects missing/invalid ones", () => {
    expect(canSeek(item(1))).toBe(true);
    const readonly = { ...item(1), seekable: false };
    expect(reliableTimeline(readonly)).toBe(true);
    expect(canSeek(readonly)).toBe(false);
    for (const value of [
      { ...item(1), duration: 0 },
      { ...item(1), duration: null },
      { ...item(1), position: NaN },
      { ...item(1), duration: 604800 },
    ]) {
      expect(reliableTimeline(value)).toBe(false);
      expect(canSeek(value)).toBe(false);
    }
  });
});

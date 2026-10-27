import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { createElement } from "react";
import {
  Artwork,
  PaperStack,
  Progress,
  artworkVisible,
} from "../src/components/Ambient";
import {
  ambientItem,
  canCollapse,
  paperCount,
  reliableTimeline,
  type MediaItem,
} from "../src/services/media";
import { defaults } from "../src/types";
const song: MediaItem = {
  id: 7,
  source: "Brave",
  title: "Fixture track",
  artist: "Fixture artist",
  subtitle: "",
  album: "",
  artwork: "data:image/png;base64,fixture",
  status: 4,
  play: true,
  pause: true,
  toggle: false,
  seekable: true,
  next: false,
  previous: false,
  sampledAt: Date.now(),
  position: 10,
  duration: 120,
  rate: 1,
  error: null,
};
const state = vi.hoisted(() => ({
  media: null as any,
  notes: null as any,
  orbit: null as any,
}));
vi.mock("../src/hooks/useMedia", () => ({ useMedia: () => state.media }));
vi.mock("../src/hooks/useNotes", () => ({ useNotes: () => state.notes }));
vi.mock("../src/hooks/useOrbit", () => ({ useOrbit: () => state.orbit }));
import App from "../src/App";
beforeEach(() => {
  state.media = {
    item: song,
    valid: true,
    busy: false,
    error: "",
    selected: null,
    select: vi.fn(),
    act: vi.fn(),
    data: { revision: 1, currentId: 7, error: null, sessions: [song] },
  };
  state.notes = {
    notes: [],
    active: 0,
    ready: true,
    busy: false,
    error: "",
    mutate: vi.fn(),
  };
  state.orbit = {
    settings: defaults,
    count: 0,
    currentHeight: 400,
    view: "ambient",
    visible: true,
    battery: {
      present: true,
      percent: 80,
      plugged: true,
      charging: false,
      remainingSeconds: null,
    },
    snapshot: {
      system: {
        cpu: 3,
        ramPercent: 48,
        ramUsed: 3e9,
        ramTotal: 8e9,
        network: true,
      },
      volume: { percent: 31, muted: false },
    },
    notification: null,
    error: "",
    setView: vi.fn(),
    setEditing: vi.fn(),
    setFocused: vi.fn(),
    cancelExpand: vi.fn(),
    scheduleExpand: vi.fn(),
    cancelCollapse: vi.fn(),
    scheduleCollapse: vi.fn(),
    report: vi.fn(),
    dismiss: vi.fn(),
    change: vi.fn(),
    measure: vi.fn(),
    performanceResult: "",
  };
});
describe("ambient artwork and media", () => {
  it("renders supplied artwork at compact size with paused indication", () => {
    const html = renderToStaticMarkup(
      createElement(Artwork, { src: song.artwork, small: true, paused: true }),
    );
    expect(html).toContain("Media artwork");
    expect(html).toContain("small");
    expect(html).toContain('aria-label="Paused"');
  });
  it("uses a music fallback for absent or failed artwork", () => {
    expect(artworkVisible(null, null)).toBe(false);
    expect(artworkVisible("broken", "broken")).toBe(false);
    expect(artworkVisible("new", "broken")).toBe(true);
    const html = renderToStaticMarkup(createElement(Artwork, { src: null }));
    expect(html).toContain("Artwork unavailable");
    expect(html).not.toContain("<img");
  });
  it("preserves paused media and recognizes ended or removed sessions", () => {
    expect(ambientItem(song)).toBe(song);
    expect(ambientItem({ ...song, status: 5 })?.status).toBe(5);
    expect(ambientItem({ ...song, status: 6 })).toBeNull();
    expect(ambientItem(undefined)).toBeNull();
  });
  it("hides zero and unknown timelines", () => {
    for (const item of [
      { ...song, duration: 0 },
      { ...song, duration: null },
    ]) {
      expect(reliableTimeline(item)).toBe(false);
      expect(
        renderToStaticMarkup(
          createElement(Progress, { item, active: false, thin: true }),
        ),
      ).toBe("");
    }
  });
  it("renders a bounded native timeline and no extra height label in thin mode", () => {
    const html = renderToStaticMarkup(
      createElement(Progress, { item: song, active: false, thin: true }),
    );
    expect(html).toContain("<progress");
    expect(html).toContain("ambient-progress");
    expect(html).not.toContain("track-progress");
  });
});
describe("unified island rendering", () => {
  it("combines media, system, volume and quick notes with no tab navigation", () => {
    const html = renderToStaticMarkup(createElement(App));
    expect(html).toContain("Fixture track");
    expect(html).toContain("CPU");
    expect(html).toContain("System volume: 31%");
    expect(html).toContain("Write a quick note...");
    expect(html).not.toContain("<nav");
    expect(html).not.toContain("module-tabs");
    expect(html).not.toContain('role="tab"');
  });
  it("has no empty media section when no session is active", () => {
    state.media.item = null;
    const html = renderToStaticMarkup(createElement(App));
    expect(html).not.toContain('aria-label="Media playback"');
    expect(html).toContain("Quick note");
  });
  it("keeps real artwork in the collapsed island and honors the artwork setting", () => {
    state.orbit.view = "closed";
    let html = renderToStaticMarkup(createElement(App));
    expect(html).toContain("artwork small");
    state.orbit.settings = { ...defaults, collapsedArtwork: false };
    html = renderToStaticMarkup(createElement(App));
    expect(html).not.toContain("artwork small");
  });
  it("keeps drafts mounted under a priority warning", () => {
    state.orbit.notification = {
      title: "Battery critical",
      detail: "Below 10%",
      priority: 100,
    };
    const html = renderToStaticMarkup(createElement(App));
    expect(html).toContain('role="alert"');
    expect(html).toMatch(/class="unified-panel" hidden/);
    expect(html).toContain("Write a quick note...");
  });
  it("renders saved note text without interpreting markup", () => {
    state.notes.notes = [
      {
        id: "fixture",
        text: "<script>test</script>",
        createdAt: 1,
        updatedAt: 1,
        completed: false,
      },
    ];
    state.notes.active = 1;
    const html = renderToStaticMarkup(createElement(App));
    expect(html).toContain("&lt;script&gt;test&lt;/script&gt;");
    expect(html).toContain("Complete note");
    expect(html).toContain("Edit saved note");
    expect(html).toContain("Delete note");
  });
});
describe("paper and interaction policies", () => {
  it("caps the physical stack at three while keeping the total count", () => {
    expect([0, 1, 2, 3, 5].map(paperCount)).toEqual([0, 1, 2, 3, 3]);
    const html = renderToStaticMarkup(
      createElement(PaperStack, {
        count: 5,
        active: 5,
        animate: false,
        open: () => {},
      }),
    );
    expect((html.match(/<button/g) || []).length).toBe(3);
    expect(html).toContain("Show 5 active notes");
  });
  it("uses a finite entrance and honors the note-animation preference", () => {
    const render = (animate: boolean) =>
      renderToStaticMarkup(
        createElement(PaperStack, {
          count: 1,
          active: 1,
          animate,
          open: () => {},
        }),
      );
    expect(render(true)).toContain("translateY(10px)");
    expect(render(false)).not.toContain("translateY(10px)");
  });
  it("has no continuous animation or paper when no active notes exist", () => {
    expect(
      renderToStaticMarkup(
        createElement(PaperStack, {
          count: 0,
          active: 0,
          animate: true,
          open: () => {},
        }),
      ),
    ).not.toContain("<button");
  });
  it("holds the island while typing, editing, keyboard-focused or manipulating controls", () => {
    expect(canCollapse(false, false, false)).toBe(true);
    for (const bits of [
      [true, false, false],
      [false, true, false],
      [false, false, true],
      [true, true, true],
    ])
      expect(canCollapse(...(bits as [boolean, boolean, boolean]))).toBe(false);
  });
});

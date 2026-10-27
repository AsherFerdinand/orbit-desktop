import { expect, it, vi } from "vitest";
import type { MediaItem } from "../src/services/media";
const harness = vi.hoisted(() => ({
  writes: [] as unknown[],
  effects: [] as Array<() => void | (() => void)>,
}));
vi.mock("react", async (importOriginal) => {
  const actual = await importOriginal<typeof import("react")>();
  return {
    ...actual,
    memo: (component: unknown) => component,
    useRef: (value: unknown) => ({ current: value }),
    useState: (value: unknown) => [
      value,
      (next: unknown) => harness.writes.push(next),
    ],
    useMemo: (fn: () => unknown) => fn(),
    useEffect: (fn: () => void) => harness.effects.push(fn),
  };
});
import { Progress } from "../src/components/Ambient";
it("previews locally, sends one seek on release and clears preview/interaction after rejection", async () => {
  const item: MediaItem = {
    id: 1,
    source: "Brave",
    title: "Track",
    artist: "Artist",
    subtitle: "",
    album: "",
    artwork: null,
    status: 5,
    play: true,
    pause: true,
    toggle: false,
    next: false,
    previous: false,
    seekable: true,
    position: 20,
    duration: 300,
    rate: 1,
    sampledAt: Date.now(),
    error: null,
  };
  let reject!: (reason: Error) => void;
  const seek = vi.fn(
    () =>
      new Promise<void>((_, fail) => {
        reject = fail;
      }),
  );
  const interaction = vi.fn();
  const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
  try {
    // Exercise the real pointer handlers without a browser or visual assertions.
    const render = Progress as unknown as (props: unknown) => any;
    const tree = render({ item, active: true, seek, interaction });
    const handlers = tree.props.children[1].props;
    const target = {
      getBoundingClientRect: () => ({ left: 10, width: 100 }),
      setPointerCapture: vi.fn(),
      hasPointerCapture: () => true,
      releasePointerCapture: vi.fn(),
    };
    const pointer = (clientX: number) => ({
      pointerId: 7,
      button: 0,
      clientX,
      currentTarget: target,
      preventDefault: vi.fn(),
    });
    handlers.onPointerDown(pointer(35));
    handlers.onPointerMove(pointer(110));
    expect(harness.writes.at(-1)).toBe(300);
    expect(seek).not.toHaveBeenCalled();
    handlers.onPointerUp(pointer(60));
    handlers.onLostPointerCapture(pointer(60));
    expect(seek).toHaveBeenCalledExactlyOnceWith(150);
    expect(harness.writes.at(-1)).toBe(150);
    expect(interaction).toHaveBeenLastCalledWith(true);
    reject(new Error("Session declined seek"));
    await Promise.resolve();
    await Promise.resolve();
    expect(harness.writes.at(-1)).toBeNull();
    expect(interaction).toHaveBeenLastCalledWith(false);
    expect(warn).toHaveBeenCalledOnce();
  } finally {
    warn.mockRestore();
  }
});

import { expect, it, vi } from "vitest";
import { artworkCache } from "../src/services/media";
it("coalesces artwork reads, retains failed results and bounds recent covers", async () => {
  const fetch = vi.fn(async (id: number) => {
    if (id === 2) throw new Error("unavailable");
    return "cover-" + id;
  });
  const load = artworkCache(fetch);
  const pending = load(1);
  expect(load(1)).toBe(pending);
  expect(await pending).toBe("cover-1");
  expect(await load(1)).toBe("cover-1");
  expect(fetch).toHaveBeenCalledTimes(1);
  expect(await load(2)).toBeNull();
  expect(await load(2)).toBeNull();
  expect(fetch).toHaveBeenCalledTimes(2);
  for (const id of [3, 4, 5]) await load(id);
  await load(2); // failed entry is still retained
  expect(fetch).toHaveBeenCalledTimes(5);
  await load(1); // oldest entry was evicted
  expect(fetch).toHaveBeenCalledTimes(6);
});

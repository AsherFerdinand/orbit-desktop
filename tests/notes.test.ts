import { describe, expect, it } from "vitest";
import { reminderInput, reminderTimestamp } from "../src/services/notes";
describe("local reminder times", () => {
  it("roundtrips local date/time to an absolute timestamp and supports no reminder", () => {
    const value = "2030-01-02T09:00";
    const at = reminderTimestamp(value, null, new Date(2029, 0, 1).getTime())!;
    expect(at).toBe(new Date(2030, 0, 2, 9, 0).getTime());
    expect(reminderInput(at)).toBe(value);
    expect(reminderTimestamp("")).toBeNull();
    expect(reminderInput(null)).toBe("");
  });
  it("rejects invalid, normalized or past new times", () => {
    const now = new Date(2030, 0, 3).getTime();
    for (const value of ["invalid", "2030-02-30T09:00", "2030-01-02T09:00"]) {
      expect(() => reminderTimestamp(value, null, now)).toThrow();
    }
  });
  it("keeps an unchanged past occurrence intact while editing note text", () => {
    const at = new Date(2030, 0, 2, 9, 0, 31).getTime();
    expect(reminderTimestamp(reminderInput(at), at, at + 600_000)).toBe(at);
    expect(() =>
      reminderTimestamp("2030-01-02T09:01", at, at + 600_000),
    ).toThrow();
  });
});

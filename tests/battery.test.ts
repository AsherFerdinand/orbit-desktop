import { describe, it, expect } from "vitest";
import { batteryLabel, remainingLabel, type Battery } from "../src/types";
const b: Battery = {
  present: true,
  percent: 48,
  plugged: false,
  charging: false,
  remainingSeconds: 7320,
};
describe("honest battery states", () => {
  it("distinguishes absent, unknown and failed API", () => {
    expect(batteryLabel(null)).toBe("Battery unavailable");
    expect(batteryLabel({ ...b, present: false })).toBe("No system battery");
    expect(batteryLabel({ ...b, present: null })).toBe(
      "Battery status unknown",
    );
  });
  it("distinguishes charging from connected", () => {
    expect(batteryLabel({ ...b, plugged: true })).toBe("Connected to power");
    expect(batteryLabel({ ...b, charging: true })).toBe("Charging");
    expect(batteryLabel({ ...b, plugged: true, percent: 100 })).toBe(
      "Fully charged",
    );
  });
  it("never invents time estimates", () => {
    expect(remainingLabel(null)).toContain("estimating");
    expect(remainingLabel(7320)).toBe("2h 2m estimated remaining");
  });
});

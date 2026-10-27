export interface Battery {
  present: boolean | null;
  percent: number | null;
  plugged: boolean | null;
  charging: boolean | null;
  remainingSeconds: number | null;
}
export interface Settings {
  collapsedArtwork: boolean;
  playbackProgress: boolean;
  quickNotes: boolean;
  paperStack: boolean;
  noteAnimations: boolean;
  compactMetrics: boolean;
  theme: "dark" | "light" | "system";
  width: number;
  height: number;
  collapseDelay: number; // Seconds; converted to milliseconds only at timer scheduling.
  animationSpeed: number;
  batteryEnabled: boolean;
  systemEnabled: boolean;
  volumeNotifications: boolean;
  chargingNotifications: boolean;
  networkNotifications: boolean;
  refreshInterval: number;
  notificationDuration: number;
  lowBatteryThreshold: number;
}
export const defaults: Settings = {
  collapsedArtwork: true,
  playbackProgress: true,
  quickNotes: true,
  paperStack: true,
  noteAnimations: true,
  compactMetrics: true,
  theme: "system",
  width: 420,
  height: 56,
  collapseDelay: 0.4,
  animationSpeed: 1,
  batteryEnabled: true,
  systemEnabled: true,
  volumeNotifications: true,
  chargingNotifications: true,
  networkNotifications: true,
  refreshInterval: 2,
  notificationDuration: 4,
  lowBatteryThreshold: 20,
};
export function batteryLabel(b: Battery | null): string {
  if (!b) return "Battery unavailable";
  if (b.present === false) return "No system battery";
  if (b.present === null) return "Battery status unknown";
  if (b.charging) return "Charging";
  if (b.plugged)
    return b.percent === 100 ? "Fully charged" : "Connected to power";
  return b.plugged === false ? "On battery power" : "Power source unknown";
}
export function remainingLabel(seconds: number | null): string {
  if (seconds === null) return "Windows is estimating time remaining";
  const m = Math.floor(seconds / 60);
  return Math.floor(m / 60) + "h " + (m % 60) + "m estimated remaining";
}

export interface Volume {
  percent: number;
  muted: boolean;
}
export interface Snapshot {
  battery: Battery | null;
  system: {
    cpu: number | null;
    ramUsed: number | null;
    ramTotal: number | null;
    ramPercent: number | null;
    network: boolean | null;
  };
  volume: Volume | null;
  audioError: string | null;
}

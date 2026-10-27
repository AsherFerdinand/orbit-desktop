// datetime-local displays Windows local time; persistence uses Unix milliseconds.
export function reminderInput(at: number | null | undefined): string {
  if (at == null) return "";
  const date = new Date(at);
  if (!Number.isFinite(date.getTime())) return "";
  const pad = (n: number) => String(n).padStart(2, "0");
  return (
    date.getFullYear() +
    "-" +
    pad(date.getMonth() + 1) +
    "-" +
    pad(date.getDate()) +
    "T" +
    pad(date.getHours()) +
    ":" +
    pad(date.getMinutes())
  );
}
export function reminderTimestamp(
  value: string,
  original: number | null = null,
  now = Date.now(),
): number | null {
  if (!value) return null;
  if (original !== null && value === reminderInput(original)) return original;
  const at = new Date(value).getTime();
  if (!Number.isFinite(at) || reminderInput(at) !== value)
    throw new Error("Choose a valid local date and time.");
  if (at <= now) throw new Error("Choose a future reminder time.");
  return at;
}

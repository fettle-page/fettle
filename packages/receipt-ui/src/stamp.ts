// The one formatting the browser still does itself: a share page shows the
// scan time in the viewer's own zone once mounted. test/stamp.test.ts keeps it equal to the Rust
// receipt in UTC.

/**
 * The scan time as "07/10/2026 22:29", in `timeZone`, or in the viewer's own zone when it is null.
 * Reports store UTC, which doesn't reveal where the PC is; whoever reads one sees their own time.
 */
export function stamp(rfc3339: string, _localeTag: string, timeZone: string | null = null): string {
  const date = new Date(rfc3339);
  if (Number.isNaN(date.getTime())) return rfc3339;
  const parts = new Intl.DateTimeFormat("en-GB", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    hourCycle: "h23",
    ...(timeZone ? { timeZone } : {}),
  }).formatToParts(date);
  const part = (type: Intl.DateTimeFormatPartTypes) => parts.find((p) => p.type === type)?.value ?? "";
  const text = `${part("day")}/${part("month")}/${part("year")} ${part("hour")}:${part("minute")}`;
  // UTC is only asked for where the viewer is unknown (server-drawn pages), so it is labelled.
  return timeZone === "UTC" ? `${text} UTC` : text;
}

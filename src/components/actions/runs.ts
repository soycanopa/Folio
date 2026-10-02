import type { ActionRunInfo } from "../../types";

/*
 * Portado de Pages CMS (https://github.com/hunvreus/pagescms) — MIT License.
 * Su formatActionRunState (lib/actions.ts) y el tiempo relativo de su
 * tabla de corridas (formatDistanceToNowStrict → helper propio, sin deps).
 */

export function formatRunState(run: Pick<ActionRunInfo, "status" | "conclusion">): string {
  if (run.status !== "completed") {
    if (
      run.status === "queued" ||
      run.status === "requested" ||
      run.status === "waiting" ||
      run.status === "dispatching" ||
      run.status == null
    ) {
      return "Queued";
    }
    return "Running";
  }
  switch (run.conclusion) {
    case "success":
      return "Succeeded";
    case "failure":
      return "Failed";
    case "cancelled":
      return "Cancelled";
    case "timed_out":
      return "Timed out";
    case "skipped":
      return "Skipped";
    default:
      return "Completed";
  }
}

export function isRunActive(run: Pick<ActionRunInfo, "status">): boolean {
  return run.status !== "completed";
}

/// "3m ago" / "2h ago" / "5d ago" — lo que su formatDistanceToNowStrict
/// strict muestra; fuera de rango, la fecha corta.
export function relativeTime(iso?: string | null): string {
  if (!iso) return "Unknown time";
  const then = Date.parse(iso);
  if (Number.isNaN(then)) return "Unknown time";
  const secs = Math.max(0, Math.floor((Date.now() - then) / 1000));
  if (secs < 60) return "just now";
  const mins = Math.floor(secs / 60);
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  if (days < 30) return `${days}d ago`;
  return new Date(then).toLocaleDateString();
}

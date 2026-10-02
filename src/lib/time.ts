// Tiempo relativo, como el formatDistanceToNow de su app.
export function relTime(unixSec: number): string {
  if (!unixSec) return "";
  const s = Math.floor(Date.now() / 1000) - unixSec;
  if (s < 45) return "just now";
  if (s < 3600) return `${Math.max(1, Math.floor(s / 60))}m ago`;
  if (s < 86400) return `${Math.floor(s / 3600)}h ago`;
  if (s < 86400 * 30) return `${Math.floor(s / 86400)}d ago`;
  return new Date(unixSec * 1000).toLocaleDateString();
}

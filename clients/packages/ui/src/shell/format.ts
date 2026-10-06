export function formatDuration(ms: number): string {
  if (!Number.isFinite(ms) || ms < 0) {
    return '0:00';
  }
  const totalSeconds = Math.floor(ms / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  const padded = seconds < 10 ? `0${seconds}` : `${seconds}`;
  return `${minutes}:${padded}`;
}

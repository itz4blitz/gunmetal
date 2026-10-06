export function artistInitial(name: string): string {
  const trimmed = name.trim();
  if (trimmed.length === 0) {
    return '?';
  }
  return trimmed.slice(0, 1).toUpperCase();
}

export function staggerSlot(index: number): string {
  return `${Math.min(Math.max(index, 0), 6)}`;
}

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

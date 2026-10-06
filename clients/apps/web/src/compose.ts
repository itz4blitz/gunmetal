export type ShellProps = {
  wordmark: string;
  title: string;
  hint: string;
  clock: string;
};

export function clockLabel(seconds: number): string {
  const minutes = Math.floor(seconds / 60);
  const remainder = seconds - minutes * 60;
  if (remainder < 10) {
    return `${minutes}:0${remainder}`;
  }
  return `${minutes}:${remainder}`;
}

export function shellProps(seconds: number): ShellProps {
  return {
    wordmark: 'Gunmetal',
    title: 'Nothing is playing',
    hint: 'The web client is running. Library and playback are not wired yet.',
    clock: clockLabel(seconds),
  };
}

export function nextCount(count: number): number {
  return count + 1;
}

export function tokenClass(): 'token-text' {
  return 'token-text';
}

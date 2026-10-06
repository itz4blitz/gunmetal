export type ShellProps = { label: string; count: number };

export function shellProps(count: number): ShellProps {
  return { label: 'Gunmetal', count };
}

export function nextCount(count: number): number {
  return count + 1;
}

export function tokenClass(): 'token-text' {
  return 'token-text';
}

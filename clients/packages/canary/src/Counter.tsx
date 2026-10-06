type Props = { label: string; count: number };

// The canary's component: one element a test finds by its role and accessible name, one value that
// changes, and one condition. Its text and its name are given to it; it holds no interface text itself.
export function Counter({ label, count }: Props) {
  return (
    <output aria-label={label} data-empty={count === 0}>
      {count}
    </output>
  );
}

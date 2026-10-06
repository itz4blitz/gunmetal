import { useEffect, useState } from 'react';
import { nextCount, shellProps } from './compose.ts';
import { Shell } from './Shell.tsx';

const tickMs = 1000;

export function App({ start = 0 }: { start?: number }) {
  const [count, setCount] = useState(start);
  useEffect(() => {
    const id = globalThis.setInterval(() => {
      setCount(nextCount);
    }, tickMs);
    return () => {
      globalThis.clearInterval(id);
    };
  }, []);
  return <Shell {...shellProps(count)} />;
}

import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { afterEach, expect, test, vi } from 'vitest';
import { LyricsPane, PROGRAMMATIC_SCROLL_GRACE_MS } from './LyricsPane.tsx';
import { FOLLOW_IDLE_MS, type SyncedLine } from './synced-lyrics.ts';

afterEach(cleanup);

const VERSE = ['First line stays lit', 'the rest wait their turn', 'no clock in the demo'];

const TIMED: readonly SyncedLine[] = [
  { atMs: 0, text: 'First line stays lit' },
  { atMs: 4_000, text: 'the rest wait their turn' },
  { atMs: 9_000, text: 'no clock in the demo' },
];

/* jsdom has no scrollIntoView; the tests give every line a recorded one so
   whichever line becomes current answers the pane's request. */
function attachScrollIntoView(root: HTMLElement, scrolled: string[]): void {
  for (const line of root.querySelectorAll('[data-lyrics-line="1"]')) {
    Object.defineProperty(line, 'scrollIntoView', {
      configurable: true,
      value: (options: unknown) => {
        scrolled.push(JSON.stringify(options));
      },
    });
  }
}

async function areaLyricsCss(): Promise<string> {
  return readFile(join(process.cwd(), 'apps/demo/public/area-lyrics.css'), 'utf8');
}

test('lyrics pane stays unmounted until open and paints lines as Text', () => {
  const closed = render(
    <LyricsPane id="lyrics-pane" label="Lyrics" lines={['First', 'Second']} synced={false} open={false} />,
  );
  expect(document.querySelector('#lyrics-pane')).toBeNull();
  closed.unmount();

  const { container } = render(
    <LyricsPane id="lyrics-pane" label="Lyrics" lines={['This file has no lyrics.']} synced={false} open />,
  );
  expect(screen.getByRole('region', { name: 'Lyrics' }).id).toStrictEqual('lyrics-pane');
  expect(container.querySelector('#lyrics-pane')?.getAttribute('data-synced')).toStrictEqual('0');
  expect([...container.querySelectorAll('[data-lyrics-line="1"]')].map((node) => node.textContent)).toStrictEqual([
    'This file has no lyrics.',
  ]);
  expect(container.querySelector('[data-current="1"]')).toBeNull();
});

test('synced lyrics highlight only the first line as a static demo', () => {
  const { container } = render(<LyricsPane id="player-full-lyrics" label="Lyrics" lines={VERSE} synced open />);
  const lines = [...container.querySelectorAll('[data-lyrics-line="1"]')];
  expect(container.querySelector('#player-full-lyrics')?.getAttribute('data-synced')).toStrictEqual('1');
  expect(lines.map((node) => node.textContent)).toStrictEqual([...VERSE]);
  expect(lines.map((node) => node.getAttribute('data-current'))).toStrictEqual(['1', '0', '0']);
});

test('synced lines carry a bounded distance from the current line', () => {
  const { container } = render(
    <LyricsPane
      id="lyrics-pane"
      label="Lyrics"
      lines={['one', 'two', 'three', 'four', 'five', 'six', 'seven', 'eight']}
      synced
      open
      currentLine={2}
    />,
  );
  const lines = [...container.querySelectorAll('[data-lyrics-line="1"]')];
  expect(lines.map((node) => node.getAttribute('data-current'))).toStrictEqual([
    '0',
    '0',
    '1',
    '0',
    '0',
    '0',
    '0',
    '0',
  ]);
  expect(lines.map((node) => node.getAttribute('data-distance'))).toStrictEqual([
    '2',
    '1',
    '0',
    '1',
    '2',
    '3',
    '4',
    '4',
  ]);
});

test('currentLine defaults to the first line; an out-of-range line lights nothing', () => {
  const { container } = render(<LyricsPane id="lyrics-pane" label="Lyrics" lines={VERSE} synced open />);
  expect(
    [...container.querySelectorAll('[data-lyrics-line="1"]')].map((node) => node.getAttribute('data-distance')),
  ).toStrictEqual(['0', '1', '2']);

  const dimmed = render(<LyricsPane id="lyrics-pane" label="Lyrics" lines={VERSE} synced open currentLine={9} />);
  expect(dimmed.container.querySelectorAll('[data-current="1"]')).toHaveLength(0);
  expect(
    [...dimmed.container.querySelectorAll('[data-lyrics-line="1"]')].map((node) => node.getAttribute('data-distance')),
  ).toStrictEqual(['4', '4', '4']);
});

test('plain lyrics carry no distance stepping', () => {
  const { container } = render(<LyricsPane id="lyrics-pane" label="Lyrics" lines={VERSE} synced={false} open />);
  expect(
    [...container.querySelectorAll('[data-lyrics-line="1"]')].map((node) => node.getAttribute('data-distance')),
  ).toStrictEqual(['0', '0', '0']);
});

test('the no-lyrics placeholder renders as quiet chrome, still plain text', () => {
  const { container } = render(
    <LyricsPane id="lyrics-pane" label="Lyrics" lines={['This file has no lyrics.']} synced={false} open empty />,
  );
  const pane = container.querySelector('#lyrics-pane');
  expect(pane?.getAttribute('data-empty')).toStrictEqual('1');
  expect(pane?.getAttribute('data-synced')).toStrictEqual('0');
  expect([...container.querySelectorAll('[data-lyrics-line="1"]')].map((node) => node.textContent)).toStrictEqual([
    'This file has no lyrics.',
  ]);
  expect(container.querySelector('[data-current="1"]')).toBeNull();
});

test('empty defaults to the reading state', () => {
  const { container } = render(<LyricsPane id="lyrics-pane" label="Lyrics" lines={VERSE} synced={false} open />);
  expect(container.querySelector('#lyrics-pane')?.getAttribute('data-empty')).toStrictEqual('0');
});

test('timed lines drive the highlighted line from the playing position', () => {
  const view = (positionMs: number) => (
    <LyricsPane id="lyrics-pane" label="Lyrics" lines={[]} synced open timedLines={TIMED} positionMs={positionMs} />
  );
  const { container, rerender } = render(view(0));
  const currents = () =>
    [...container.querySelectorAll('[data-lyrics-line="1"]')].map((node) => node.getAttribute('data-current'));
  expect(currents()).toStrictEqual(['1', '0', '0']);
  expect([...container.querySelectorAll('[data-lyrics-line="1"]')].map((node) => node.textContent)).toStrictEqual([
    'First line stays lit',
    'the rest wait their turn',
    'no clock in the demo',
  ]);
  rerender(view(4_000));
  expect(currents()).toStrictEqual(['0', '1', '0']);
  rerender(view(60_000));
  expect(currents()).toStrictEqual(['0', '0', '1']);
  // Before the first timestamp nothing is lit yet.
  rerender(
    <LyricsPane
      id="lyrics-pane"
      label="Lyrics"
      lines={[]}
      synced
      open
      timedLines={[{ atMs: 2_000, text: 'starts late' }]}
      positionMs={0}
    />,
  );
  expect(container.querySelector('[data-current="1"]')).toBeNull();
});

test('a plain pane ignores timed lines and keeps its own text', () => {
  const { container } = render(
    <LyricsPane id="lyrics-pane" label="Lyrics" lines={['plain only']} synced={false} open timedLines={TIMED} />,
  );
  expect([...container.querySelectorAll('[data-lyrics-line="1"]')].map((node) => node.textContent)).toStrictEqual([
    'plain only',
  ]);
  expect(container.querySelector('[data-current="1"]')).toBeNull();
});

test('a wheel or touch pause stops the follow, and the idle delay resumes it', () => {
  vi.useFakeTimers();
  try {
    const { container } = render(
      <LyricsPane id="lyrics-pane" label="Lyrics" lines={[]} synced open timedLines={TIMED} positionMs={4_000} />,
    );
    const scroller = container.querySelector('[data-lyrics-scroll="1"]') as HTMLElement;
    expect(scroller.getAttribute('data-lyrics-scroll')).toStrictEqual('1');
    expect(scroller.getAttribute('data-following')).toStrictEqual('1');
    const wait = (ms: number) => {
      act(() => {
        vi.advanceTimersByTime(ms);
      });
    };
    // A manual wheel pauses the follow…
    fireEvent.wheel(scroller, { deltaY: 120 });
    expect(scroller.getAttribute('data-following')).toStrictEqual('0');
    // …and the idle delay brings it back.
    wait(FOLLOW_IDLE_MS - 1);
    expect(scroller.getAttribute('data-following')).toStrictEqual('0');
    wait(1);
    expect(scroller.getAttribute('data-following')).toStrictEqual('1');
    // A second gesture restarts the wait rather than stacking two.
    fireEvent.wheel(scroller, { deltaY: 10 });
    wait(FOLLOW_IDLE_MS - 1);
    expect(scroller.getAttribute('data-following')).toStrictEqual('0');
    // A touch carries a real touch object: the responder reads its force.
    fireEvent.touchMove(scroller, {
      changedTouches: [{ identifier: 1, clientX: 10, clientY: 10, force: 0, pageX: 10, pageY: 10 }],
      touches: [{ identifier: 1, clientX: 10, clientY: 10, force: 0, pageX: 10, pageY: 10 }],
    });
    wait(1);
    expect(scroller.getAttribute('data-following')).toStrictEqual('0');
    wait(FOLLOW_IDLE_MS - 1);
    expect(scroller.getAttribute('data-following')).toStrictEqual('1');
    // The pane's own idle delay is honoured.
    const slow = render(
      <LyricsPane id="lyrics-pane" label="Lyrics" lines={[]} synced open timedLines={TIMED} followIdleMs={100} />,
    );
    const slowScroller = slow.container.querySelector('[data-lyrics-scroll="1"]') as HTMLElement;
    fireEvent.wheel(slowScroller, { deltaY: 5 });
    wait(100);
    expect(slowScroller.getAttribute('data-following')).toStrictEqual('1');
  } finally {
    vi.useRealTimers();
  }
});

test('the programmatic scroll grace window is a stated constant', () => {
  expect(PROGRAMMATIC_SCROLL_GRACE_MS).toStrictEqual(500);
});

test('a position tick inside the sounding line does not re-scroll the pane', () => {
  const scrolled: string[] = [];
  const view = (positionMs: number) => (
    <LyricsPane id="lyrics-pane" label="Lyrics" lines={[]} synced open timedLines={TIMED} positionMs={positionMs} />
  );
  const { container, rerender } = render(view(0));
  attachScrollIntoView(container, scrolled);
  // The same first line, later in time: nothing new to bring into view.
  rerender(view(3_999));
  expect(scrolled).toStrictEqual([]);
  // The second line starts: exactly one programmatic scroll.
  rerender(view(4_000));
  expect(scrolled).toStrictEqual(['{"block":"center"}']);
  // The same second line again: no second scroll.
  rerender(view(8_999));
  expect(scrolled).toStrictEqual(['{"block":"center"}']);
  // The third line starts: one more.
  rerender(view(9_000));
  expect(scrolled).toStrictEqual(['{"block":"center"}', '{"block":"center"}']);
  // The rest of the track stays on the third line: no further scrolls.
  rerender(view(60_000));
  expect(scrolled).toStrictEqual(['{"block":"center"}', '{"block":"center"}']);
});

test('a rebuilt timed sheet with the same sounding line does not re-scroll the pane', () => {
  const scrolled: string[] = [];
  const view = (positionMs: number) => (
    <LyricsPane
      id="lyrics-pane"
      label="Lyrics"
      lines={[]}
      synced
      open
      timedLines={[...TIMED]}
      positionMs={positionMs}
    />
  );
  const { container, rerender } = render(view(0));
  attachScrollIntoView(container, scrolled);
  // The parent hands over a fresh array on every render; the same sounding
  // line must not re-run the follow effect (only the line index may).
  rerender(view(1_000));
  rerender(view(3_999));
  expect(scrolled).toStrictEqual([]);
  // A new sounding line still scrolls exactly once, however fresh the array.
  rerender(view(4_000));
  expect(scrolled).toStrictEqual(['{"block":"center"}']);
  rerender(view(5_000));
  expect(scrolled).toStrictEqual(['{"block":"center"}']);
});

test('a synced sheet without timestamps keeps its static highlight and never scrolls it', () => {
  const scrolled: string[] = [];
  const view = (currentLine: number) => (
    <LyricsPane id="lyrics-pane" label="Lyrics" lines={VERSE} synced open currentLine={currentLine} />
  );
  const { container, rerender } = render(view(0));
  attachScrollIntoView(container, scrolled);
  // The prop moves the highlight, but without a timed sheet the pane owns
  // no clock, so it must not drive the scroller.
  rerender(view(1));
  expect(
    [...container.querySelectorAll('[data-lyrics-line="1"]')].map((node) => node.getAttribute('data-current')),
  ).toStrictEqual(['0', '1', '0']);
  expect(scrolled).toStrictEqual([]);
  rerender(view(2));
  expect(scrolled).toStrictEqual([]);
});

test('while following, the current line is brought into view, and our own scroll is not a manual one', () => {
  vi.useFakeTimers();
  try {
    const scrolled: string[] = [];
    const view = (positionMs: number) => (
      <LyricsPane id="lyrics-pane" label="Lyrics" lines={[]} synced open timedLines={TIMED} positionMs={positionMs} />
    );
    const { container, rerender } = render(view(4_000));
    const scroller = container.querySelector('[data-lyrics-scroll="1"]') as HTMLElement;
    attachScrollIntoView(container, scrolled);
    // The first line of this render was already lit without the capability;
    // moving to the next line asks the pane to bring it into view.
    rerender(view(9_000));
    expect(scrolled).toStrictEqual(['{"block":"center"}']);
    // A smooth scroll reports a whole burst of scroll events: every one of
    // them is the pane's own, so none of them cancels the follow. This is
    // the regression the fix closes; the follow used to cancel itself here.
    fireEvent.scroll(scroller);
    fireEvent.scroll(scroller);
    fireEvent.scroll(scroller);
    expect(scroller.getAttribute('data-following')).toStrictEqual('1');
    // Once the grace window has run out, a scroll with no programmatic cause
    // is a manual one and pauses the follow.
    act(() => {
      vi.advanceTimersByTime(PROGRAMMATIC_SCROLL_GRACE_MS);
    });
    fireEvent.scroll(scroller);
    expect(scroller.getAttribute('data-following')).toStrictEqual('0');
    // The idle delay resumes the follow and brings the sounding line back.
    act(() => {
      vi.advanceTimersByTime(FOLLOW_IDLE_MS);
    });
    expect(scroller.getAttribute('data-following')).toStrictEqual('1');
    expect(scrolled).toStrictEqual(['{"block":"center"}', '{"block":"center"}']);
  } finally {
    vi.useRealTimers();
  }
});

test('without a scroll capability the pane keeps its follow honestly', () => {
  const { container } = render(
    <LyricsPane id="lyrics-pane" label="Lyrics" lines={[]} synced open timedLines={TIMED} positionMs={0} />,
  );
  const scroller = container.querySelector('[data-lyrics-scroll="1"]') as HTMLElement;
  // No scrollIntoView exists here: the guard returns and nothing breaks.
  expect(scroller.getAttribute('data-following')).toStrictEqual('1');
});

test('area-lyrics.css draws the pane from type and colour tokens only', async () => {
  const css = await areaLyricsCss();
  expect(css.includes("[data-lyrics-pane='1']")).toStrictEqual(true);
  expect(css.includes('[data-lyrics-line')).toStrictEqual(true);
  expect(css.includes('var(--gm-type-title3-size)')).toStrictEqual(true);
  expect(css.includes('var(--gm-type-title3-line)')).toStrictEqual(true);
  expect(css.includes('var(--gm-text-primary)')).toStrictEqual(true);
  expect(css.includes('var(--gm-text-secondary)')).toStrictEqual(true);
  expect(css.includes('var(--gm-text-muted)')).toStrictEqual(true);
  // 2026 pass: plain lyrics read one ramp step above the synced base.
  expect(css.includes('calc(var(--gm-type-title3-size) + 1px)')).toStrictEqual(true);
  // The synced current line carries a brass where-you-are bar (§3: where you are).
  expect(css.includes('var(--gm-accent-indicator)')).toStrictEqual(true);
  // The column fades faintly at its ends to suggest continuation — a mask,
  // drawn from token colours, never an image.
  expect(css.includes('mask-image')).toStrictEqual(true);
  // No raw palette values, no assets, no emoji: tokens and text only.
  expect(/#[0-9a-fA-F]{3,8}\b/.test(css)).toStrictEqual(false);
  expect(css.includes('url(')).toStrictEqual(false);
  expect(/\p{Extended_Pictographic}/u.test(css)).toStrictEqual(false);
});

test('the active-line change runs on the motion token and stops under reduced motion', async () => {
  const css = await areaLyricsCss();
  expect(css.includes('var(--gm-motion)')).toStrictEqual(true);
  expect(css.includes('@media (prefers-reduced-motion: reduce)')).toStrictEqual(true);
  expect(css.includes('transition: none')).toStrictEqual(true);
  // The where-you-are bar fades on the same token and dies with it.
  expect(css.includes("[data-lyrics-line='1']::before")).toStrictEqual(true);
  expect(css.includes("[data-lyrics-line='1']::before {\n    transition: none")).toStrictEqual(true);
});

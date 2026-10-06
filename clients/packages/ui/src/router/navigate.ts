import type { HistoryState } from './match.ts';

export type HistoryWriter = {
  pushState: (state: HistoryState, unused: string, url: string) => void;
  replaceState: (state: HistoryState, unused: string, url: string) => void;
};

export function historyPayload(scrollY: number, itemId: string | undefined = undefined): HistoryState {
  return { scrollY, itemId };
}

export function pushPath(
  history: HistoryWriter,
  path: string,
  scrollY = 0,
  itemId: string | undefined = undefined,
): HistoryState {
  const state = historyPayload(scrollY, itemId);
  history.pushState(state, '', path);
  return state;
}

export function replacePath(
  history: HistoryWriter,
  path: string,
  scrollY = 0,
  itemId: string | undefined = undefined,
): HistoryState {
  const state = historyPayload(scrollY, itemId);
  history.replaceState(state, '', path);
  return state;
}

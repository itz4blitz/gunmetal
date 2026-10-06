import type { HistoryState } from './match.ts';

export type HistoryWriter = {
  pushState: (state: HistoryState, unused: string, url: string) => void;
  replaceState: (state: HistoryState, unused: string, url: string) => void;
};

export function historyPayload(scrollY: number): HistoryState {
  return { scrollY, itemId: undefined };
}

export function pushPath(history: HistoryWriter, path: string, scrollY = 0): HistoryState {
  const state = historyPayload(scrollY);
  history.pushState(state, '', path);
  return state;
}

export function replacePath(history: HistoryWriter, path: string, scrollY = 0): HistoryState {
  const state = historyPayload(scrollY);
  history.replaceState(state, '', path);
  return state;
}

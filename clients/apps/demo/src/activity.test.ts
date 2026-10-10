import { expect, test } from 'vitest';
import { activityCount, activityFromDocument, activityPercent } from './activity.ts';

test('an activity document is the job list', () => {
  expect(
    activityFromDocument({
      jobs: [{ id: 'artwork', label: 'Fetching album art', done: 2, total: 8 }],
    }),
  ).toStrictEqual({
    jobs: [{ id: 'artwork', label: 'Fetching album art', done: 2, total: 8 }],
  });
  expect(activityFromDocument({ jobs: [] })).toStrictEqual({ jobs: [] });
});

test('a bad activity document is refused', () => {
  expect(activityFromDocument(null)).toStrictEqual(undefined);
  expect(activityFromDocument({ jobs: [{ id: '', label: 'Fetching album art', done: 1, total: 2 }] })).toStrictEqual(
    undefined,
  );
  expect(
    activityFromDocument({ jobs: [{ id: 'artwork', label: 'Fetching album art', done: 3, total: 2 }] }),
  ).toStrictEqual(undefined);
  expect(activityFromDocument({ jobs: 'nope' })).toStrictEqual(undefined);
  // Every guard in one job row: shape, id, label, integers, range.
  expect(activityFromDocument({ jobs: [42] })).toStrictEqual(undefined);
  expect(activityFromDocument({ jobs: [{ id: `a${'b'.repeat(33)}`, label: 'x', done: 0, total: 1 }] })).toStrictEqual(
    undefined,
  );
  expect(activityFromDocument({ jobs: [{ id: 'a\0b', label: 'x', done: 0, total: 1 }] })).toStrictEqual(undefined);
  expect(activityFromDocument({ jobs: [{ id: 'artwork', label: '', done: 0, total: 1 }] })).toStrictEqual(undefined);
  expect(
    activityFromDocument({ jobs: [{ id: 'artwork', label: `x${'y'.repeat(80)}`, done: 0, total: 1 }] }),
  ).toStrictEqual(undefined);
  expect(activityFromDocument({ jobs: [{ id: 'artwork', label: 'a\0b', done: 0, total: 1 }] })).toStrictEqual(
    undefined,
  );
  expect(activityFromDocument({ jobs: [{ id: 'artwork', label: 'x', done: 0.5, total: 1 }] })).toStrictEqual(undefined);
  expect(activityFromDocument({ jobs: [{ id: 'artwork', label: 'x', done: 0, total: 0 }] })).toStrictEqual(undefined);
  expect(activityFromDocument({ jobs: [{ id: 'artwork', label: 'x', done: 0, total: 2001 }] })).toStrictEqual(
    undefined,
  );
  expect(activityFromDocument({ jobs: 'nope' })).toStrictEqual(undefined);
  expect(
    activityFromDocument({ jobs: new Array(9).fill({ id: 'artwork', label: 'x', done: 0, total: 1 }) }),
  ).toStrictEqual(undefined);
});

test('progress is the share of the job that is done', () => {
  const row = { id: 'artists', label: 'Fetching artist photos', done: 1, total: 4 };
  expect(activityPercent(row)).toStrictEqual(25);
  expect(activityCount([row, { ...row, done: 2 }])).toStrictEqual(3);
});

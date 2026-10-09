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
  expect(activityFromDocument({ jobs: [{ id: 'artwork', label: 'Fetching album art', done: 3, total: 2 }] })).toStrictEqual(
    undefined,
  );
  expect(activityFromDocument({ jobs: 'nope' })).toStrictEqual(undefined);
});

test('progress is the share of the job that is done', () => {
  const row = { id: 'artists', label: 'Fetching artist photos', done: 1, total: 4 };
  expect(activityPercent(row)).toStrictEqual(25);
  expect(activityCount([row, { ...row, done: 2 }])).toStrictEqual(3);
});

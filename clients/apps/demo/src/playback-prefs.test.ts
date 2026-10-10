import { expect, test } from 'vitest';
import {
  defaultPlaybackPrefs,
  readPlaybackPrefs,
  serializePlaybackPrefs,
  type PlaybackPrefs,
} from './playback-prefs.ts';

const fallback: PlaybackPrefs = { levelling: 'off', crossfadeSeconds: 0, sinkId: '' };

test('a fresh preference is levelling off, no crossfade and no sink', () => {
  expect(defaultPlaybackPrefs()).toStrictEqual(fallback);
  const first = defaultPlaybackPrefs();
  first.levelling = 'album';
  first.crossfadeSeconds = 12;
  first.sinkId = 'changed';
  expect(defaultPlaybackPrefs()).toStrictEqual(fallback);
});

test('nothing stored, or JSON that is not a preference object, reads as the default', () => {
  expect(readPlaybackPrefs(null)).toStrictEqual(fallback);
  expect(readPlaybackPrefs('')).toStrictEqual(fallback);
  expect(readPlaybackPrefs('   ')).toStrictEqual(fallback);
  expect(readPlaybackPrefs('not json')).toStrictEqual(fallback);
  expect(readPlaybackPrefs('{')).toStrictEqual(fallback);
  expect(readPlaybackPrefs('null')).toStrictEqual(fallback);
  expect(readPlaybackPrefs('true')).toStrictEqual(fallback);
  expect(readPlaybackPrefs('false')).toStrictEqual(fallback);
  expect(readPlaybackPrefs('0')).toStrictEqual(fallback);
  expect(readPlaybackPrefs('"prefs"')).toStrictEqual(fallback);
  expect(readPlaybackPrefs('[]')).toStrictEqual(fallback);
  expect(readPlaybackPrefs('[{"levelling":"album","crossfadeSeconds":4,"sinkId":"desk"}]')).toStrictEqual(fallback);
  const read = readPlaybackPrefs(null);
  read.levelling = 'track';
  expect(readPlaybackPrefs(null)).toStrictEqual(fallback);
});

test('a field that is not an allowed value falls back on its own', () => {
  expect(readPlaybackPrefs('{}')).toStrictEqual(fallback);
  expect(readPlaybackPrefs('{"crossfadeSeconds":4,"sinkId":"desk"}')).toStrictEqual({
    levelling: 'off',
    crossfadeSeconds: 4,
    sinkId: 'desk',
  });
  expect(readPlaybackPrefs('{"levelling":"nope","crossfadeSeconds":6,"sinkId":"desk"}')).toStrictEqual({
    levelling: 'off',
    crossfadeSeconds: 6,
    sinkId: 'desk',
  });
  expect(readPlaybackPrefs('{"levelling":"Off","crossfadeSeconds":8,"sinkId":"desk"}')).toStrictEqual({
    levelling: 'off',
    crossfadeSeconds: 8,
    sinkId: 'desk',
  });
  expect(readPlaybackPrefs('{"levelling":"ALBUM","crossfadeSeconds":2,"sinkId":"desk"}')).toStrictEqual({
    levelling: 'off',
    crossfadeSeconds: 2,
    sinkId: 'desk',
  });
  expect(readPlaybackPrefs('{"levelling":"album ","crossfadeSeconds":12,"sinkId":"desk"}')).toStrictEqual({
    levelling: 'off',
    crossfadeSeconds: 12,
    sinkId: 'desk',
  });
  expect(readPlaybackPrefs('{"levelling":" track","crossfadeSeconds":4,"sinkId":"desk"}')).toStrictEqual({
    levelling: 'off',
    crossfadeSeconds: 4,
    sinkId: 'desk',
  });
  expect(readPlaybackPrefs('{"levelling":"","crossfadeSeconds":4,"sinkId":"desk"}')).toStrictEqual({
    levelling: 'off',
    crossfadeSeconds: 4,
    sinkId: 'desk',
  });
  expect(readPlaybackPrefs('{"levelling":1,"crossfadeSeconds":12,"sinkId":"desk"}')).toStrictEqual({
    levelling: 'off',
    crossfadeSeconds: 12,
    sinkId: 'desk',
  });
  expect(readPlaybackPrefs('{"levelling":true,"crossfadeSeconds":2,"sinkId":"desk"}')).toStrictEqual({
    levelling: 'off',
    crossfadeSeconds: 2,
    sinkId: 'desk',
  });
  expect(readPlaybackPrefs('{"levelling":null,"crossfadeSeconds":6,"sinkId":"desk"}')).toStrictEqual({
    levelling: 'off',
    crossfadeSeconds: 6,
    sinkId: 'desk',
  });
  expect(readPlaybackPrefs('{"levelling":"album","sinkId":"desk"}')).toStrictEqual({
    levelling: 'album',
    crossfadeSeconds: 0,
    sinkId: 'desk',
  });
  expect(readPlaybackPrefs('{"levelling":"album","crossfadeSeconds":1,"sinkId":"desk"}')).toStrictEqual({
    levelling: 'album',
    crossfadeSeconds: 0,
    sinkId: 'desk',
  });
  expect(readPlaybackPrefs('{"levelling":"album","crossfadeSeconds":10,"sinkId":"desk"}')).toStrictEqual({
    levelling: 'album',
    crossfadeSeconds: 0,
    sinkId: 'desk',
  });
  expect(readPlaybackPrefs('{"levelling":"album","crossfadeSeconds":14,"sinkId":"desk"}')).toStrictEqual({
    levelling: 'album',
    crossfadeSeconds: 0,
    sinkId: 'desk',
  });
  expect(readPlaybackPrefs('{"levelling":"album","crossfadeSeconds":16,"sinkId":"desk"}')).toStrictEqual({
    levelling: 'album',
    crossfadeSeconds: 0,
    sinkId: 'desk',
  });
  expect(readPlaybackPrefs('{"levelling":"track","crossfadeSeconds":-2,"sinkId":"desk"}')).toStrictEqual({
    levelling: 'track',
    crossfadeSeconds: 0,
    sinkId: 'desk',
  });
  expect(readPlaybackPrefs('{"levelling":"track","crossfadeSeconds":0.5,"sinkId":"desk"}')).toStrictEqual({
    levelling: 'track',
    crossfadeSeconds: 0,
    sinkId: 'desk',
  });
  expect(readPlaybackPrefs('{"levelling":"track","crossfadeSeconds":3,"sinkId":"desk"}')).toStrictEqual({
    levelling: 'track',
    crossfadeSeconds: 0,
    sinkId: 'desk',
  });
  expect(readPlaybackPrefs('{"levelling":"track","crossfadeSeconds":"4","sinkId":"desk"}')).toStrictEqual({
    levelling: 'track',
    crossfadeSeconds: 0,
    sinkId: 'desk',
  });
  expect(readPlaybackPrefs('{"levelling":"track","crossfadeSeconds":true,"sinkId":"desk"}')).toStrictEqual({
    levelling: 'track',
    crossfadeSeconds: 0,
    sinkId: 'desk',
  });
  expect(readPlaybackPrefs('{"levelling":"track","crossfadeSeconds":false,"sinkId":"desk"}')).toStrictEqual({
    levelling: 'track',
    crossfadeSeconds: 0,
    sinkId: 'desk',
  });
  expect(readPlaybackPrefs('{"levelling":"track","crossfadeSeconds":null,"sinkId":"desk"}')).toStrictEqual({
    levelling: 'track',
    crossfadeSeconds: 0,
    sinkId: 'desk',
  });
  expect(readPlaybackPrefs('{"levelling":"album","crossfadeSeconds":4}')).toStrictEqual({
    levelling: 'album',
    crossfadeSeconds: 4,
    sinkId: '',
  });
  expect(readPlaybackPrefs('{"levelling":"album","crossfadeSeconds":4,"sinkId":1}')).toStrictEqual({
    levelling: 'album',
    crossfadeSeconds: 4,
    sinkId: '',
  });
  expect(readPlaybackPrefs('{"levelling":"album","crossfadeSeconds":4,"sinkId":null}')).toStrictEqual({
    levelling: 'album',
    crossfadeSeconds: 4,
    sinkId: '',
  });
  expect(readPlaybackPrefs('{"levelling":"album","crossfadeSeconds":4,"sinkId":true}')).toStrictEqual({
    levelling: 'album',
    crossfadeSeconds: 4,
    sinkId: '',
  });
  expect(readPlaybackPrefs('{"levelling":"album","crossfadeSeconds":4,"sinkId":{"id":"desk"}}')).toStrictEqual({
    levelling: 'album',
    crossfadeSeconds: 4,
    sinkId: '',
  });
  expect(readPlaybackPrefs('{"levelling":"nope","crossfadeSeconds":3,"sinkId":1}')).toStrictEqual(fallback);
  // -0 is not a distinct duration. It reads as no crossfade, and the other fields stay.
  expect(readPlaybackPrefs('{"levelling":"track","crossfadeSeconds":-0,"sinkId":"desk"}')).toStrictEqual({
    levelling: 'track',
    crossfadeSeconds: 0,
    sinkId: 'desk',
  });
});

test('each allowed levelling and crossfade is kept, and other keys are ignored', () => {
  expect(readPlaybackPrefs('{"sinkId":"a","crossfadeSeconds":0,"levelling":"off"}')).toStrictEqual({
    levelling: 'off',
    crossfadeSeconds: 0,
    sinkId: 'a',
  });
  expect(readPlaybackPrefs('{"levelling":"track","crossfadeSeconds":2,"sinkId":"a"}')).toStrictEqual({
    levelling: 'track',
    crossfadeSeconds: 2,
    sinkId: 'a',
  });
  expect(readPlaybackPrefs('{"levelling":"album","crossfadeSeconds":4,"sinkId":"a","token":"secret"}')).toStrictEqual({
    levelling: 'album',
    crossfadeSeconds: 4,
    sinkId: 'a',
  });
  expect(readPlaybackPrefs('{"levelling":"album","crossfadeSeconds":6,"sinkId":"a"}')).toStrictEqual({
    levelling: 'album',
    crossfadeSeconds: 6,
    sinkId: 'a',
  });
  expect(readPlaybackPrefs('{"levelling":"track","crossfadeSeconds":8,"sinkId":"a"}')).toStrictEqual({
    levelling: 'track',
    crossfadeSeconds: 8,
    sinkId: 'a',
  });
  expect(readPlaybackPrefs('{"levelling":"track","crossfadeSeconds":12,"sinkId":"a"}')).toStrictEqual({
    levelling: 'track',
    crossfadeSeconds: 12,
    sinkId: 'a',
  });
  expect(readPlaybackPrefs('{"levelling":"\\u0061lbum","crossfadeSeconds":4,"sinkId":"\\u0061"}')).toStrictEqual({
    levelling: 'album',
    crossfadeSeconds: 4,
    sinkId: 'a',
  });
  expect(readPlaybackPrefs('\n{"levelling":"track","crossfadeSeconds":2,"sinkId":"desk"}\n')).toStrictEqual({
    levelling: 'track',
    crossfadeSeconds: 2,
    sinkId: 'desk',
  });
  expect(
    readPlaybackPrefs('{"__proto__":{"levelling":"album"},"levelling":"track","crossfadeSeconds":2,"sinkId":"desk"}'),
  ).toStrictEqual({
    levelling: 'track',
    crossfadeSeconds: 2,
    sinkId: 'desk',
  });
  const raw = '{"levelling":"album","crossfadeSeconds":4,"sinkId":"desk"}';
  const first = readPlaybackPrefs(raw);
  first.sinkId = 'changed';
  expect(readPlaybackPrefs(raw)).toStrictEqual({ levelling: 'album', crossfadeSeconds: 4, sinkId: 'desk' });
});

test('a sink of 200 code units is kept; a longer sink or one containing NUL is refused', () => {
  const twoHundred = 'a'.repeat(200);
  const twoHundredOne = 'a'.repeat(201);
  const hundredEmoji = '\u{1F3A7}'.repeat(100);
  const twoHundredEmoji = '\u{1F3A7}'.repeat(200);
  expect(readPlaybackPrefs('{"levelling":"track","crossfadeSeconds":2,"sinkId":""}')).toStrictEqual({
    levelling: 'track',
    crossfadeSeconds: 2,
    sinkId: '',
  });
  expect(readPlaybackPrefs('{"levelling":"track","crossfadeSeconds":2,"sinkId":"   "}')).toStrictEqual({
    levelling: 'track',
    crossfadeSeconds: 2,
    sinkId: '   ',
  });
  expect(
    readPlaybackPrefs(JSON.stringify({ levelling: 'track', crossfadeSeconds: 2, sinkId: twoHundred })),
  ).toStrictEqual({
    levelling: 'track',
    crossfadeSeconds: 2,
    sinkId: twoHundred,
  });
  expect(
    readPlaybackPrefs(JSON.stringify({ levelling: 'track', crossfadeSeconds: 2, sinkId: twoHundredOne })),
  ).toStrictEqual({
    levelling: 'track',
    crossfadeSeconds: 2,
    sinkId: '',
  });
  expect(
    readPlaybackPrefs(JSON.stringify({ levelling: 'track', crossfadeSeconds: 2, sinkId: hundredEmoji })),
  ).toStrictEqual({
    levelling: 'track',
    crossfadeSeconds: 2,
    sinkId: hundredEmoji,
  });
  expect(
    readPlaybackPrefs(JSON.stringify({ levelling: 'album', crossfadeSeconds: 4, sinkId: twoHundredEmoji })),
  ).toStrictEqual({
    levelling: 'album',
    crossfadeSeconds: 4,
    sinkId: '',
  });
  expect(readPlaybackPrefs('{"levelling":"album","crossfadeSeconds":4,"sinkId":"\\u0000"}')).toStrictEqual({
    levelling: 'album',
    crossfadeSeconds: 4,
    sinkId: '',
  });
  expect(readPlaybackPrefs('{"levelling":"album","crossfadeSeconds":4,"sinkId":"a\\u0000b"}')).toStrictEqual({
    levelling: 'album',
    crossfadeSeconds: 4,
    sinkId: '',
  });
  expect(readPlaybackPrefs('{"levelling":"album","crossfadeSeconds":4,"sinkId":"\\u0000desk"}')).toStrictEqual({
    levelling: 'album',
    crossfadeSeconds: 4,
    sinkId: '',
  });
  expect(readPlaybackPrefs('{"levelling":"album","crossfadeSeconds":4,"sinkId":"desk\\u0000"}')).toStrictEqual({
    levelling: 'album',
    crossfadeSeconds: 4,
    sinkId: '',
  });
  expect(
    readPlaybackPrefs(JSON.stringify({ levelling: 'album', crossfadeSeconds: 4, sinkId: `${'a'.repeat(199)}\0` })),
  ).toStrictEqual({
    levelling: 'album',
    crossfadeSeconds: 4,
    sinkId: '',
  });
  expect(readPlaybackPrefs('{"levelling":"album","crossfadeSeconds":6,"sinkId":"line\\nbreak"}')).toStrictEqual({
    levelling: 'album',
    crossfadeSeconds: 6,
    sinkId: 'line\nbreak',
  });
});

test('the preference is written as levelling, crossfade seconds and sink, and nothing else', () => {
  const prefs: PlaybackPrefs = { levelling: 'album', crossfadeSeconds: 8, sinkId: 'speakers' };
  expect(serializePlaybackPrefs(prefs)).toStrictEqual('{"levelling":"album","crossfadeSeconds":8,"sinkId":"speakers"}');
  expect(serializePlaybackPrefs(defaultPlaybackPrefs())).toStrictEqual(
    '{"levelling":"off","crossfadeSeconds":0,"sinkId":""}',
  );
  expect(readPlaybackPrefs(serializePlaybackPrefs(prefs))).toStrictEqual(prefs);
  expect(readPlaybackPrefs(serializePlaybackPrefs(defaultPlaybackPrefs()))).toStrictEqual(fallback);
  const withSecret = { levelling: 'track', crossfadeSeconds: 4, sinkId: 'desk', token: 'secret' } as PlaybackPrefs;
  expect(serializePlaybackPrefs(withSecret)).toStrictEqual(
    '{"levelling":"track","crossfadeSeconds":4,"sinkId":"desk"}',
  );
});

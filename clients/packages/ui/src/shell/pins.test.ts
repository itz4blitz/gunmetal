import { expect, test } from 'vitest';
import { readPins, serializePins, togglePin, type Pin } from './pins.ts';

const home: Pin = { path: '/', label: 'Home' };
const search: Pin = { path: '/search', label: 'Search' };
const library: Pin = { path: '/library', label: 'Library' };

test('nothing stored, or JSON that is not a list of pins, reads as no pins', () => {
  expect(readPins(null)).toStrictEqual([]);
  expect(readPins('')).toStrictEqual([]);
  expect(readPins('not json')).toStrictEqual([]);
  expect(readPins('{')).toStrictEqual([]);
  expect(readPins('null')).toStrictEqual([]);
  expect(readPins('true')).toStrictEqual([]);
  expect(readPins('false')).toStrictEqual([]);
  expect(readPins('0')).toStrictEqual([]);
  expect(readPins('"[]"')).toStrictEqual([]);
  expect(readPins('{}')).toStrictEqual([]);
  expect(readPins('{"path":"/","label":"Home"}')).toStrictEqual([]);
  expect(readPins('[]')).toStrictEqual([]);
});

test('entries that are not pins are dropped and a real pin stays', () => {
  expect(
    readPins(
      JSON.stringify([
        null,
        1,
        true,
        'pin',
        [],
        ['/search', 'Search'],
        {},
        { path: '/search' },
        { label: 'Search' },
        { path: 1, label: 'Search' },
        { path: '/search', label: 1 },
        { path: '/search', label: true },
        { path: '/search', label: null },
        { path: null, label: 'Search' },
        search,
      ]),
    ),
  ).toStrictEqual([search]);
});

test('only the closed route list can be pinned, and a stored pin is path and label alone', () => {
  expect(
    readPins(
      JSON.stringify([
        { path: '/settings/privacy', label: 'Privacy', token: 'secret' },
        { path: '/nope', label: 'Nope' },
        { path: '/settings/library', label: 'Not a section' },
        { path: '//evil.example/search', label: 'Off site relative' },
        { path: 'https://evil.example/search', label: 'Off site' },
        { path: '/search/', label: 'Slash' },
        { path: '/Search', label: 'Case' },
        { path: 'search', label: 'Bare' },
        { path: '/settings/appearance-extra', label: 'Extra' },
        { path: '/library/albums', label: 'Albums' },
        { path: '', label: 'Empty' },
        { label: 'Home', path: '/' },
      ]),
    ),
  ).toStrictEqual([{ path: '/settings/privacy', label: 'Privacy' }, home]);
  // A decoded escape is the path, not the raw spelling.
  expect(readPins('[{"path":"\\u002fsearch","label":"Search"}]')).toStrictEqual([search]);
  expect(readPins('\n[ {"path": "/library", "label": "Library"} ]\n')).toStrictEqual([library]);
});

test('every allowed path is kept in the order it was stored', () => {
  const stored: readonly Pin[] = [
    { path: '/settings/privacy', label: 'Privacy' },
    { path: '/settings/about', label: 'About' },
    { path: '/settings/extensions', label: 'Extensions' },
    { path: '/settings/playback', label: 'Playback' },
    { path: '/settings/appearance', label: 'Appearance' },
    { path: '/settings', label: 'Settings' },
    library,
    search,
    home,
  ];
  expect(readPins(JSON.stringify(stored))).toStrictEqual([
    { path: '/settings/privacy', label: 'Privacy' },
    { path: '/settings/about', label: 'About' },
    { path: '/settings/extensions', label: 'Extensions' },
    { path: '/settings/playback', label: 'Playback' },
    { path: '/settings/appearance', label: 'Appearance' },
    { path: '/settings', label: 'Settings' },
    library,
    search,
    home,
  ]);
});

test('a label of 80 characters is kept; a longer label or one containing NUL is dropped', () => {
  const eighty = 'a'.repeat(80);
  const eightyOne = 'a'.repeat(81);
  expect(readPins(JSON.stringify([{ path: '/search', label: '' }]))).toStrictEqual([{ path: '/search', label: '' }]);
  expect(readPins(JSON.stringify([{ path: '/search', label: '   ' }]))).toStrictEqual([
    { path: '/search', label: '   ' },
  ]);
  expect(readPins(JSON.stringify([{ path: '/search', label: eighty }]))).toStrictEqual([
    { path: '/search', label: eighty },
  ]);
  expect(readPins(JSON.stringify([{ path: '/search', label: eightyOne }]))).toStrictEqual([]);
  expect(readPins(JSON.stringify([{ path: '/search', label: '\0' }]))).toStrictEqual([]);
  expect(readPins(JSON.stringify([{ path: '/search', label: 'a\0b' }]))).toStrictEqual([]);
  expect(readPins(JSON.stringify([{ path: '/search', label: '\0Search' }]))).toStrictEqual([]);
  expect(readPins(JSON.stringify([{ path: '/search', label: 'Search\0' }]))).toStrictEqual([]);
  expect(readPins('[{"path":"/search","label":"a\\u0000b"}]')).toStrictEqual([]);
  // A newline is not NUL. The limit counts code units, so one emoji is well under 80.
  expect(readPins(JSON.stringify([{ path: '/search', label: 'line\nbreak' }]))).toStrictEqual([
    { path: '/search', label: 'line\nbreak' },
  ]);
  expect(readPins(JSON.stringify([{ path: '/search', label: 'Search 🔍' }]))).toStrictEqual([
    { path: '/search', label: 'Search 🔍' },
  ]);
});

test('a duplicate path is dropped, unless the earlier copy was itself dropped', () => {
  expect(
    readPins(
      JSON.stringify([
        { path: '/search', label: 'First' },
        { path: '/library', label: 'Library' },
        { path: '/search', label: 'Second' },
      ]),
    ),
  ).toStrictEqual([{ path: '/search', label: 'First' }, library]);
  expect(
    readPins(
      JSON.stringify([
        { path: '/search', label: 'a'.repeat(81) },
        { path: '/search', label: 'Search' },
      ]),
    ),
  ).toStrictEqual([search]);
  expect(readPins(JSON.stringify([{ path: '/nope', label: 'Nope' }, search]))).toStrictEqual([search]);
  // `/settings` and a section under it are different pins.
  expect(
    readPins(
      JSON.stringify([
        { path: '/settings', label: 'Settings' },
        { path: '/settings/appearance', label: 'Appearance' },
      ]),
    ),
  ).toStrictEqual([
    { path: '/settings', label: 'Settings' },
    { path: '/settings/appearance', label: 'Appearance' },
  ]);
});

test('each read is a new array', () => {
  expect(readPins(null)).toStrictEqual([]);
  expect(readPins(null)).not.toBe(readPins(null));
  const once = readPins(JSON.stringify([home]));
  const twice = readPins(JSON.stringify([home]));
  expect(once).toStrictEqual([home]);
  expect(twice).toStrictEqual([home]);
  expect(once).not.toBe(twice);
});

test('togglePin adds a missing pin and removes one that is already pinned, by path', () => {
  const pins: Pin[] = [home, search];
  const added = togglePin(pins, library);
  expect(added).toStrictEqual([home, search, library]);
  expect(pins).toStrictEqual([home, search]);

  const removed = togglePin(added, { path: '/search', label: 'Other' });
  expect(removed).toStrictEqual([home, library]);
  expect(added).toStrictEqual([home, search, library]);
  expect(togglePin([search], search)).toStrictEqual([]);
  expect(togglePin([], search)).toStrictEqual([search]);

  const doubled: Pin[] = [home, { path: '/settings', label: 'Settings' }, search, { path: '/search', label: 'Again' }];
  expect(togglePin(doubled, { path: '/search', label: 'Search' })).toStrictEqual([
    home,
    { path: '/settings', label: 'Settings' },
  ]);
  expect(doubled).toStrictEqual([
    home,
    { path: '/settings', label: 'Settings' },
    search,
    { path: '/search', label: 'Again' },
  ]);
  expect(
    togglePin(
      [
        { path: '/settings', label: 'Settings' },
        { path: '/settings/appearance', label: 'Appearance' },
      ],
      { path: '/settings', label: 'Settings' },
    ),
  ).toStrictEqual([{ path: '/settings/appearance', label: 'Appearance' }]);
});

test('togglePin copies the pin it adds, so a later edit of the argument is not a stored edit', () => {
  const pins: Pin[] = [home];
  const pin: Pin = { path: '/library', label: 'Library' };
  const next = togglePin(pins, pin);
  pin.label = 'Changed';
  expect(next).toStrictEqual([home, library]);
  expect(pins).toStrictEqual([home]);
});

test('togglePin refuses a path that is not on the closed list and does not mutate the input', () => {
  const pins: Pin[] = [home, search];
  expect(togglePin(pins, { path: '/nope', label: 'Nope' })).toStrictEqual([home, search]);
  expect(togglePin(pins, { path: '/settings/library', label: 'Not a section' })).toStrictEqual([home, search]);
  expect(togglePin(pins, { path: '//evil.example/', label: 'Off site relative' })).toStrictEqual([home, search]);
  expect(togglePin(pins, { path: 'https://evil.example/', label: 'Off site' })).toStrictEqual([home, search]);
  expect(togglePin(pins, { path: '', label: 'Empty' })).toStrictEqual([home, search]);
  expect(togglePin(pins, { path: '/search/', label: 'Slash' })).toStrictEqual([home, search]);
  expect(togglePin(pins, { path: '/Search', label: 'Case' })).toStrictEqual([home, search]);
  expect(togglePin(pins, { path: '/library/albums', label: 'Albums' })).toStrictEqual([home, search]);
  expect(pins).toStrictEqual([home, search]);
  // Refuse wins over remove: a disallowed path already in the list stays.
  const stray: Pin[] = [home, { path: '/nope', label: 'Nope' }];
  expect(togglePin(stray, { path: '/nope', label: 'Other' })).toStrictEqual([home, { path: '/nope', label: 'Nope' }]);
  expect(stray).toStrictEqual([home, { path: '/nope', label: 'Nope' }]);
});

test('togglePin will add a label that a later read would drop', () => {
  const long: Pin = { path: '/search', label: 'a'.repeat(81) };
  const nul: Pin = { path: '/library', label: 'Lib\0rary' };
  expect(togglePin([], long)).toStrictEqual([long]);
  expect(togglePin([home], nul)).toStrictEqual([home, nul]);
  expect(readPins(serializePins([long, nul]))).toStrictEqual([]);
});

test('serializePins writes a JSON array of path and label, and nothing else', () => {
  const pins: Pin[] = [home, search];
  expect(serializePins([])).toStrictEqual('[]');
  expect(serializePins(pins)).toStrictEqual('[{"path":"/","label":"Home"},{"path":"/search","label":"Search"}]');
  expect(pins).toStrictEqual([home, search]);
  expect(serializePins([{ path: '/library', label: 'Library', token: 'secret' } as Pin])).toStrictEqual(
    '[{"path":"/library","label":"Library"}]',
  );
  expect(serializePins([{ path: '/not-a-route', label: 'x' }])).toStrictEqual('[{"path":"/not-a-route","label":"x"}]');
  expect(serializePins([{ path: '/search', label: 'say "hi"\\' }])).toStrictEqual(
    '[{"path":"/search","label":"say \\"hi\\"\\\\"}]',
  );
});

test('an album pin is kept beside the library page, and a bad item id is dropped', () => {
  const album: Pin = { path: '/library', label: 'Harbour Lights', itemId: 'demo-album-01' };
  const artist: Pin = { path: '/library', label: 'Mira Sol', itemId: 'mira-sol' };
  const served: Pin = { path: '/library', label: 'St. Elsewhere', itemId: 'a'.repeat(16) };
  expect(readPins(JSON.stringify([library, album, artist, served, album]))).toStrictEqual([
    library,
    album,
    artist,
    served,
  ]);
  expect(
    readPins(
      JSON.stringify([
        { path: '/library', label: 'Nope', itemId: 'Has Space' },
        { path: '/library', label: 'Nope', itemId: 'a/b' },
        { path: '/library', label: 'Nope', itemId: '' },
        { path: '/library', label: 'Nope', itemId: 'A'.repeat(16) },
        { path: '/library', label: 'Nope', itemId: 'a'.repeat(65) },
        { path: '/search', label: 'Search', itemId: 'demo-album-01' },
        { path: '/library', label: 'Nope', itemId: 1 },
        { path: '/library', label: 'Nope', itemId: null },
        album,
      ]),
    ),
  ).toStrictEqual([album]);
  expect(readPins(JSON.stringify([{ path: '/settings/connected', label: 'Connected services' }]))).toStrictEqual([
    { path: '/settings/connected', label: 'Connected services' },
  ]);
  expect(
    readPins(JSON.stringify([{ path: '/settings/extensions/cover-art', label: 'Metadata and artwork' }])),
  ).toStrictEqual([{ path: '/settings/extensions/cover-art', label: 'Metadata and artwork' }]);
  expect(readPins(JSON.stringify([{ path: '/settings/extensions/nope', label: 'Nope' }]))).toStrictEqual([]);
});

test('togglePin adds and removes an album without touching another album or the library page', () => {
  const album: Pin = { path: '/library', label: 'Harbour Lights', itemId: 'demo-album-01' };
  const other: Pin = { path: '/library', label: 'Night Shift', itemId: 'demo-album-02' };
  const added = togglePin([library], album);
  expect(added).toStrictEqual([library, album]);
  expect(togglePin(added, other)).toStrictEqual([library, album, other]);
  expect(togglePin(added, { path: '/library', label: 'Other title', itemId: 'demo-album-01' })).toStrictEqual([
    library,
  ]);
  expect(togglePin([library], { path: '/search', label: 'Search', itemId: 'demo-album-01' })).toStrictEqual([library]);
  expect(togglePin([library], { path: '/library', label: 'Nope', itemId: 'Has Space' })).toStrictEqual([library]);
  expect(togglePin([library], { path: '/library', label: 'Nope', itemId: '' })).toStrictEqual([library]);
  const argument: Pin = { path: '/library', label: 'Harbour Lights', itemId: 'demo-album-01' };
  const copied = togglePin([], argument);
  argument.label = 'Changed';
  argument.itemId = 'demo-album-02';
  expect(copied).toStrictEqual([album]);
});

test('serializePins keeps an item id and still drops every other field', () => {
  const album: Pin = { path: '/library', label: 'Harbour Lights', itemId: 'demo-album-01' };
  expect(serializePins([album])).toStrictEqual(
    '[{"path":"/library","label":"Harbour Lights","itemId":"demo-album-01"}]',
  );
  expect(readPins(serializePins([home, album]))).toStrictEqual([home, album]);
});

test('a list of allowed pins round-trips through one JSON array', () => {
  const pins: readonly Pin[] = [home, library, { path: '/settings/privacy', label: 'Privacy' }];
  expect(serializePins(pins)).toStrictEqual(
    '[{"path":"/","label":"Home"},{"path":"/library","label":"Library"},{"path":"/settings/privacy","label":"Privacy"}]',
  );
  expect(readPins(serializePins(pins))).toStrictEqual([home, library, { path: '/settings/privacy', label: 'Privacy' }]);
});

test('a media address is a pin, and a broken media address is not', () => {
  const album: Pin = { path: '/music/albums/harbour-lights', label: 'Harbour Lights' };
  const movie: Pin = { path: '/watch/movies/inception', label: 'Inception' };
  expect(togglePin([home], album)).toStrictEqual([home, album]);
  expect(readPins(serializePins([album, movie]))).toStrictEqual([album, movie]);
  expect(readPins('[{"path":"/music/albums/Harbour","label":"Nope"}]')).toStrictEqual([]);
  expect(togglePin([home], { path: '/music/albums/../secret', label: 'Nope' })).toStrictEqual([home]);
});

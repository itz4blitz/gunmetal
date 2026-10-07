import { useMemo } from 'react';
import { Text, View } from 'react-native-web';
import type { DestinationMessages } from '../../messages/en/destinations.ts';
import { artistInitial } from '../format.ts';
import { Icon } from '../Icon.tsx';
import type { ShellAlbum, ShellArtist, ShellLibrary, ShellTrack } from '../library-types.ts';
import { AlbumTile } from './AlbumTile.tsx';
import { indexAlbums } from './library-index.ts';
import { intersectionObserverFactory } from './near-view.ts';
import { TrackRow } from './TrackRow.tsx';
import { LIST_OVERSCAN_ROWS, contentScroller, useContentViewport, visibleRange } from './windowing.ts';

/** The all-songs table's row height, pinned by the area CSS row contract. */
const TRACK_ROW_HEIGHT = 52;

/** The artist page's read: the artist's albums in library order, O(1) per id. */
function albumsForArtist(albums: ReadonlyMap<string, ShellAlbum>, artist: ShellArtist): readonly ShellAlbum[] {
  const found: ShellAlbum[] = [];
  for (const id of artist.albumIds) {
    const album = albums.get(id);
    if (album !== undefined) {
      found.push(album);
    }
  }
  return found;
}

/** All songs reads across the artist's own albums; hostile albums never render. */
function tracksForArtist(albums: readonly ShellAlbum[]): readonly { album: ShellAlbum; track: ShellTrack }[] {
  const rows: { album: ShellAlbum; track: ShellTrack }[] = [];
  for (const album of albums) {
    if (album.hostile) {
      continue;
    }
    for (const track of album.tracks) {
      rows.push({ album, track });
    }
  }
  return rows;
}

/** Same-name artists (MUS-006) are told apart by their catalogue key. */
function artistSharesName(library: ShellLibrary, artist: ShellArtist): boolean {
  const name = artist.name.trim();
  if (name === '') {
    return false;
  }
  return library.artists.some((row) => row.key !== artist.key && row.name.trim() === name);
}

export type ArtistDetailProps = {
  artist: ShellArtist | undefined;
  library: ShellLibrary;
  messages: DestinationMessages;
  currentTrackId?: string | undefined;
  /**
   * The scroll container the all-songs list windows against; the shell's
   * content pane by default. Tests inject a stand-in (jsdom has no layout).
   */
  getScroller?: (() => HTMLElement | null) | undefined;
  /**
   * The album grid defers cover URLs until tiles are near the viewport. The
   * browser constructor by default; undefined (jsdom) paints immediately.
   */
  nearViewObserver?: typeof IntersectionObserver | undefined;
  onBack: () => void;
  onOpenAlbum: (albumId: string) => void;
  onPlayAlbum: (albumId: string) => void;
  onOpenArtist?: ((artistKey: string) => void) | undefined;
  onPlayTrack?: ((albumId: string, trackId: string) => void) | undefined;
  onPlayNextAlbum?: ((albumId: string) => void) | undefined;
  onAddAlbumToQueue?: ((albumId: string) => void) | undefined;
  onPlayNextTrack?: ((albumId: string, trackId: string) => void) | undefined;
  onAddTrackToQueue?: ((albumId: string, trackId: string) => void) | undefined;
};

function activateKey(event: { key: string; preventDefault: () => void }, action: () => void): void {
  if (event.key === 'Enter' || event.key === ' ') {
    event.preventDefault();
    action();
  }
}

function artistHeading(artist: ShellArtist, albums: readonly ShellAlbum[], messages: DestinationMessages): string {
  if (albums.some((album) => album.hostile)) {
    return messages.hostileArtistLabel;
  }
  return artist.name;
}

export function ArtistDetail({
  artist,
  library,
  messages,
  currentTrackId,
  getScroller,
  nearViewObserver,
  onBack,
  onOpenAlbum,
  onPlayAlbum,
  onOpenArtist,
  onPlayTrack,
  onPlayNextAlbum,
  onAddAlbumToQueue,
  onPlayNextTrack,
  onAddTrackToQueue,
}: ArtistDetailProps) {
  // One album index per library change, built before any early return.
  const albumIndex = useMemo(() => indexAlbums(library), [library]);
  // The rows the content pane can see; a zero read (jsdom) shows everything.
  const viewport = useContentViewport(getScroller ?? contentScroller);
  if (artist === undefined) {
    return (
      <View id="destination-artist-missing">
        <Text id="destination-headline" accessibilityRole="header">
          {messages.artistMissing}
        </Text>
        <View
          id="artist-back"
          accessibilityRole="button"
          accessibilityLabel={messages.backToLibrary}
          tabIndex={0}
          onClick={onBack}
          onKeyDown={(event) => {
            activateKey(event, onBack);
          }}
        >
          <Icon name="back" size={18} />
          <Text>{messages.backToLibrary}</Text>
        </View>
      </View>
    );
  }

  const albums = albumsForArtist(albumIndex, artist);
  const songs = tracksForArtist(albums);
  const songWindow = visibleRange(
    viewport.scrollTop,
    viewport.viewportHeight,
    TRACK_ROW_HEIGHT,
    songs.length,
    LIST_OVERSCAN_ROWS,
  );
  const shownSongs = songs.slice(songWindow.start, songWindow.end);
  // Undefined in jsdom: no factory, every cover paints immediately.
  const nearArt = useMemo(
    () => intersectionObserverFactory(nearViewObserver ?? globalThis.IntersectionObserver),
    [nearViewObserver],
  );
  const allSongsOpen = onPlayTrack !== undefined && songs.length > 0;
  const first = albums[0];
  const heading = artistHeading(artist, albums, messages);
  const artTone = first?.coverTone;
  const artistData = artTone === undefined ? { artistKey: artist.key } : { artistKey: artist.key, artTone };

  return (
    <View id="destination-artist" dataSet={artistData}>
      <View
        id="artist-back"
        accessibilityRole="button"
        accessibilityLabel={messages.backToLibrary}
        tabIndex={0}
        onClick={onBack}
        onKeyDown={(event) => {
          activateKey(event, onBack);
        }}
      >
        <Icon name="back" size={18} />
        <Text>{messages.backToLibrary}</Text>
      </View>
      <View dataSet={{ artistHeader: '1' }}>
        {/* Ambient bloom of the artist image behind the hero — the album
            page's move, same tokens, aria-hidden and pointer-transparent. */}
        {artist.imageUrl === undefined ? null : (
          <View
            dataSet={{ artistBloom: '1' }}
            aria-hidden={true}
            style={{ backgroundImage: `url("${artist.imageUrl}")` }}
          />
        )}
        {artist.imageUrl === undefined ? null : <View dataSet={{ artistScrim: '1' }} aria-hidden={true} />}
        <View
          dataSet={{
            artistHero: '1',
            artistAvatar: 'hero',
            artistAvatarNut: '1',
            artistImage: artist.imageUrl === undefined ? '0' : '1',
          }}
          aria-hidden={true}
          style={
            artist.imageUrl === undefined
              ? undefined
              : { backgroundImage: `url("${artist.imageUrl}")`, backgroundSize: 'cover', backgroundPosition: 'center' }
          }
        >
          <Text dataSet={{ artistInitial: '1' }}>{artistInitial(heading)}</Text>
        </View>
        <View dataSet={{ artistHeaderText: '1' }}>
          <Text dataSet={{ detailEyebrow: '1' }}>{messages.artistEyebrow}</Text>
          <Text id="destination-headline" accessibilityRole="header">
            {heading}
          </Text>
          {artistSharesName(library, artist) ? (
            <Text id="artist-disambiguation" dataSet={{ artistDisambiguation: '1' }}>
              {artist.key}
            </Text>
          ) : null}
          <View dataSet={{ artistMeta: '1' }}>
            <Text dataSet={{ artistAlbumCount: '1' }}>{`${albums.length} ${messages.artistAlbumCount}`}</Text>
            <Text dataSet={{ metaSeparator: '1' }}>·</Text>
            <Text dataSet={{ artistSongCount: '1' }}>{`${songs.length} ${messages.artistSongCount}`}</Text>
          </View>
          <View
            id="artist-play"
            accessibilityRole="button"
            accessibilityLabel={messages.play}
            tabIndex={0}
            dataSet={{ brassHex: '1' }}
            onClick={() => {
              if (first !== undefined) {
                onPlayAlbum(first.id);
              }
            }}
            onKeyDown={(event) => {
              activateKey(event, () => {
                if (first !== undefined) {
                  onPlayAlbum(first.id);
                }
              });
            }}
          >
            <Text>{messages.play}</Text>
          </View>
        </View>
      </View>
      <Text accessibilityRole="header" dataSet={{ sectionHeading: '1', type: 'title2' }}>
        {messages.tabAlbums}
      </Text>
      <View id="artist-album-grid" dataSet={{ albumGrid: '1' }}>
        {albums.map((album, index) => (
          <AlbumTile
            key={album.id}
            album={album}
            messages={messages}
            staggerIndex={index}
            nearArt={nearArt}
            onOpen={onOpenAlbum}
            onPlay={onPlayAlbum}
            onPlayNext={onPlayNextAlbum}
            onAddToQueue={onAddAlbumToQueue}
            onOpenArtist={onOpenArtist}
          />
        ))}
      </View>
      {allSongsOpen ? (
        <View id="artist-all-songs" dataSet={{ artistSongs: '1' }}>
          <Text accessibilityRole="header" dataSet={{ sectionHeading: '1', type: 'title2' }}>
            {messages.allSongsHeading}
          </Text>
          {/* Presentational column chrome; the rows stay buttons, so the
              head is aria-hidden and carries the labels for the eye only. */}
          <View dataSet={{ artistTableHead: '1' }} aria-hidden={true}>
            <Text>{messages.columnNumber}</Text>
            <Text>{messages.columnTitle}</Text>
            <Text>{messages.columnAlbum}</Text>
            <Text>{messages.columnTime}</Text>
          </View>
          <View dataSet={{ artistSongList: '1' }}>
            {shownSongs.map(({ album, track }) => (
              <TrackRow
                key={track.id}
                track={track}
                messages={messages}
                current={track.id === currentTrackId}
                artistKey={album.artistKey}
                albumTitle={album.title}
                onPlay={onPlayTrack}
                onPlayNext={onPlayNextTrack}
                onAddToQueue={onAddTrackToQueue}
                onOpenArtist={onOpenArtist}
              />
            ))}
          </View>
        </View>
      ) : null}
    </View>
  );
}

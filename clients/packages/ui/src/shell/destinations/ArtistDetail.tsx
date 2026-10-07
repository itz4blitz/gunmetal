import { Text, View } from 'react-native-web';
import type { DestinationMessages } from '../../messages/en/destinations.ts';
import { artistInitial } from '../format.ts';
import type { ShellAlbum, ShellArtist, ShellLibrary, ShellTrack } from '../library-types.ts';
import { AlbumTile } from './AlbumTile.tsx';
import { TrackRow } from './TrackRow.tsx';

/** The artist page's read: the artist's albums in library order. */
function albumsForArtist(library: ShellLibrary, artist: ShellArtist): readonly ShellAlbum[] {
  const albums: ShellAlbum[] = [];
  for (const id of artist.albumIds) {
    const found = library.albums.find((album) => album.id === id);
    if (found !== undefined) {
      albums.push(found);
    }
  }
  return albums;
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
          <Text>{messages.backToLibrary}</Text>
        </View>
      </View>
    );
  }

  const albums = albumsForArtist(library, artist);
  const songs = tracksForArtist(albums);
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
        <Text>{messages.backToLibrary}</Text>
      </View>
      <View dataSet={{ artistHeader: '1' }}>
        <View
          dataSet={{
            artistHero: '1',
            artistAvatar: 'hero',
            artistAvatarNut: '1',
            artistImage: artist.imageUrl === undefined ? '0' : '1',
          }}
          aria-hidden="true"
          style={
            artist.imageUrl === undefined
              ? undefined
              : { backgroundImage: `url("${artist.imageUrl}")`, backgroundSize: 'cover', backgroundPosition: 'center' }
          }
        >
          <Text dataSet={{ artistInitial: '1' }}>{artistInitial(heading)}</Text>
        </View>
        <View dataSet={{ artistHeaderText: '1' }}>
          <Text id="destination-headline" accessibilityRole="header">
            {heading}
          </Text>
          {artistSharesName(library, artist) ? (
            <Text id="artist-disambiguation" dataSet={{ artistDisambiguation: '1' }}>
              {artist.key}
            </Text>
          ) : null}
          <Text dataSet={{ artistAlbumCount: '1' }}>{`${albums.length} ${messages.artistAlbumCount}`}</Text>
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
      <View id="artist-album-grid" dataSet={{ albumGrid: '1' }}>
        {albums.map((album, index) => (
          <AlbumTile
            key={album.id}
            album={album}
            messages={messages}
            staggerIndex={index}
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
          <View dataSet={{ artistSongList: '1' }}>
            {songs.map(({ album, track }) => (
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

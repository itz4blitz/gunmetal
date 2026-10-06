import { useState } from 'react';
import { Text, View } from 'react-native-web';
import { demoLyricsLines, demoLyricsVerse } from '../../../../fake-server/src/lyrics.ts';
import type { DestinationMessages } from '../../messages/en/destinations.ts';
import type { ShellAlbum, ShellTrack } from '../library-types.ts';
import { LyricsPane } from '../LyricsPane.tsx';
import { CoverTile } from './CoverTile.tsx';
import { TrackRow } from './TrackRow.tsx';

function albumLyricsTrack(album: ShellAlbum, currentTrackId?: string): ShellTrack | undefined {
  const current = album.tracks.find((track) => track.id === currentTrackId);
  if (current !== undefined && (current.lyricsKind === 'plain' || current.lyricsKind === 'synced')) {
    return current;
  }
  return album.tracks.find((track) => track.lyricsKind === 'plain' || track.lyricsKind === 'synced');
}

export type AlbumDetailProps = {
  album: ShellAlbum | undefined;
  messages: DestinationMessages;
  currentTrackId?: string;
  onBack: () => void;
  onPlayAlbum: (albumId: string) => void;
  onPlayTrack: (albumId: string, trackId: string) => void;
  onOpenArtist?: (artistKey: string) => void;
  onPlayNextAlbum?: (albumId: string) => void;
  onAddAlbumToQueue?: (albumId: string) => void;
  onPlayNextTrack?: (albumId: string, trackId: string) => void;
  onAddTrackToQueue?: (albumId: string, trackId: string) => void;
};

export function AlbumDetail({
  album,
  messages,
  currentTrackId,
  onBack,
  onPlayAlbum,
  onPlayTrack,
  onOpenArtist,
  onPlayNextTrack,
  onAddTrackToQueue,
}: AlbumDetailProps) {
  const [lyricsOpen, setLyricsOpen] = useState(false);
  if (album === undefined) {
    return (
      <View id="destination-album-missing">
        <Text id="destination-headline" accessibilityRole="header">
          {messages.albumMissing}
        </Text>
        <View
          id="album-back"
          accessibilityRole="button"
          accessibilityLabel={messages.backToLibrary}
          tabIndex={0}
          onClick={onBack}
          onKeyDown={(event) => {
            if (event.key === 'Enter' || event.key === ' ') {
              event.preventDefault();
              onBack();
            }
          }}
        >
          <Text>{messages.backToLibrary}</Text>
        </View>
      </View>
    );
  }

  const title = album.hostile ? messages.hostileAlbumLabel : album.title;
  const artist = album.hostile ? messages.hostileArtistLabel : album.artistName;
  const lyricsTrack = albumLyricsTrack(album, currentTrackId);

  return (
    <View
      id="destination-album"
      dataSet={{
        albumId: album.id,
        hostile: album.hostile ? '1' : '0',
        artTone: album.coverTone,
      }}
    >
      <View
        id="album-back"
        accessibilityRole="button"
        accessibilityLabel={messages.backToLibrary}
        tabIndex={0}
        onClick={onBack}
        onKeyDown={(event) => {
          if (event.key === 'Enter' || event.key === ' ') {
            event.preventDefault();
            onBack();
          }
        }}
      >
        <Text>{messages.backToLibrary}</Text>
      </View>
      <View dataSet={{ albumHeader: '1', albumHeaderLarge: '1', albumHeaderBleed: '1' }}>
        <CoverTile
          tone={album.coverTone}
          label={title}
          size="detail"
          coverId={`cover-detail-${album.id}`}
        />
        <View dataSet={{ albumHeaderText: '1' }}>
          <Text id="destination-headline" accessibilityRole="header">
            {title}
          </Text>
          <View
            dataSet={{ albumArtist: '1', type: 'title3' }}
            accessibilityRole={onOpenArtist === undefined ? undefined : 'button'}
            accessibilityLabel={onOpenArtist === undefined ? artist : messages.goToArtist}
            tabIndex={onOpenArtist === undefined ? undefined : 0}
            onClick={() => {
              if (onOpenArtist !== undefined) {
                onOpenArtist(album.artistKey);
              }
            }}
            onKeyDown={(event) => {
              if (onOpenArtist !== undefined && (event.key === 'Enter' || event.key === ' ')) {
                event.preventDefault();
                onOpenArtist(album.artistKey);
              }
            }}
          >
            <Text>{artist}</Text>
          </View>
          <Text dataSet={{ albumYear: '1' }}>{`${messages.yearLabel} ${album.year}`}</Text>
          <Text id="album-track-count" dataSet={{ albumTrackCount: '1' }}>
            {`${album.tracks.length} ${messages.trackCountLabel}`}
          </Text>
          <View
            id="album-play"
            accessibilityRole="button"
            accessibilityLabel={messages.playAlbum}
            tabIndex={0}
            dataSet={{ brassHex: '1' }}
            onClick={() => {
              onPlayAlbum(album.id);
            }}
            onKeyDown={(event) => {
              if (event.key === 'Enter' || event.key === ' ') {
                event.preventDefault();
                onPlayAlbum(album.id);
              }
            }}
          >
            <Text>{messages.playAlbum}</Text>
          </View>
          {lyricsTrack === undefined ? null : (
            <View
              id="album-lyrics-toggle"
              accessibilityRole="button"
              accessibilityLabel={messages.lyrics}
              tabIndex={0}
              dataSet={{ lyricsToggle: lyricsOpen ? '1' : '0' }}
              onClick={() => {
                setLyricsOpen((open) => !open);
              }}
              onKeyDown={(event) => {
                if (event.key === 'Enter' || event.key === ' ') {
                  event.preventDefault();
                  setLyricsOpen((open) => !open);
                }
              }}
            >
              <Text>{messages.lyrics}</Text>
            </View>
          )}
        </View>
      </View>
      {lyricsTrack === undefined ? null : (
        <LyricsPane
          id="album-lyrics"
          label={messages.lyricsRegion}
          lines={demoLyricsLines(demoLyricsVerse(lyricsTrack.id, lyricsTrack.lyricsKind))}
          synced={lyricsTrack.lyricsKind === 'synced'}
          open={lyricsOpen}
        />
      )}
      {album.discs.length > 1
        ? album.discs.map((disc) => (
            <View key={disc.index} dataSet={{ discBlock: `${disc.index}` }}>
              <Text accessibilityRole="header" dataSet={{ discHeader: '1', type: 'title2' }}>
                {disc.title === '' ? `${messages.discsHeading} ${disc.index}` : disc.title}
              </Text>
              {album.tracks
                .filter((track) => track.discIndex === disc.index)
                .map((track) => (
                  <TrackRow
                    key={track.id}
                    track={track}
                    messages={messages}
                    current={track.id === currentTrackId}
                    artistKey={album.artistKey}
                    onPlay={onPlayTrack}
                    onPlayNext={onPlayNextTrack}
                    onAddToQueue={onAddTrackToQueue}
                    onOpenArtist={onOpenArtist}
                  />
                ))}
            </View>
          ))
        : (
          <View dataSet={{ discBlock: '1' }}>
            <Text accessibilityRole="header" dataSet={{ discHeader: '1', type: 'title2' }}>
              {messages.tracksHeading}
            </Text>
            {album.tracks.map((track) => (
              <TrackRow
                key={track.id}
                track={track}
                messages={messages}
                current={track.id === currentTrackId}
                artistKey={album.artistKey}
                onPlay={onPlayTrack}
                onPlayNext={onPlayNextTrack}
                onAddToQueue={onAddTrackToQueue}
                onOpenArtist={onOpenArtist}
              />
            ))}
          </View>
        )}
    </View>
  );
}

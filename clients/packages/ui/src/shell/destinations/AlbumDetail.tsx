import { Text, View } from 'react-native-web';
import type { DestinationMessages } from '../../messages/en/destinations.ts';
import type { ShellAlbum } from '../library-types.ts';
import { CoverTile } from './CoverTile.tsx';
import { TrackRow } from './TrackRow.tsx';

export type AlbumDetailProps = {
  album: ShellAlbum | undefined;
  messages: DestinationMessages;
  onBack: () => void;
  onPlayAlbum: (albumId: string) => void;
  onPlayTrack: (albumId: string, trackId: string) => void;
};

export function AlbumDetail({
  album,
  messages,
  onBack,
  onPlayAlbum,
  onPlayTrack,
}: AlbumDetailProps) {
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

  return (
    <View id="destination-album" dataSet={{ albumId: album.id, hostile: album.hostile ? '1' : '0' }}>
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
      <View dataSet={{ albumHeader: '1' }}>
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
          <Text dataSet={{ albumArtist: '1' }}>{artist}</Text>
          <Text dataSet={{ albumYear: '1' }}>{`${messages.yearLabel} ${album.year}`}</Text>
          <View
            id="album-play"
            accessibilityRole="button"
            accessibilityLabel={messages.playAlbum}
            tabIndex={0}
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
        </View>
      </View>
      {album.discs.length > 1
        ? album.discs.map((disc) => (
            <View key={disc.index} dataSet={{ discBlock: `${disc.index}` }}>
              <Text accessibilityRole="header">
                {disc.title === '' ? `${messages.discsHeading} ${disc.index}` : disc.title}
              </Text>
              {album.tracks
                .filter((track) => track.discIndex === disc.index)
                .map((track) => (
                  <TrackRow
                    key={track.id}
                    track={track}
                    messages={messages}
                    onPlay={onPlayTrack}
                  />
                ))}
            </View>
          ))
        : album.tracks.map((track) => (
            <TrackRow key={track.id} track={track} messages={messages} onPlay={onPlayTrack} />
          ))}
    </View>
  );
}

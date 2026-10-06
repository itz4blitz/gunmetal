import { Text, View } from 'react-native-web';
import type { DestinationMessages } from '../../messages/en/destinations.ts';
import { formatDuration } from '../format.ts';
import type { ShellTrack } from '../library-types.ts';

export type TrackRowProps = {
  track: ShellTrack;
  messages: DestinationMessages;
  current?: boolean;
  onPlay: (albumId: string, trackId: string) => void;
};

export function TrackRow({ track, messages, current = false, onPlay }: TrackRowProps) {
  const flagLabel =
    track.flag === 'unplayable'
      ? messages.trackFlagUnplayable
      : track.flag === 'damaged'
        ? messages.trackFlagDamaged
        : '';
  return (
    <View
      id={`track-row-${track.id}`}
      dataSet={{ trackRow: track.id, current: current ? '1' : '0' }}
      accessibilityRole="button"
      accessibilityLabel={track.title}
      tabIndex={0}
      onClick={() => {
        onPlay(track.albumId, track.id);
      }}
      onKeyDown={(event) => {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          onPlay(track.albumId, track.id);
        }
      }}
    >
      {current ? <View dataSet={{ nowPlaying: '1' }} /> : null}
      <Text dataSet={{ trackNumber: '1' }}>{`${track.number}`}</Text>
      <View dataSet={{ trackMeta: '1' }}>
        <Text dataSet={{ trackTitle: '1' }}>{track.title}</Text>
        <Text dataSet={{ trackArtist: '1' }}>{track.artistName}</Text>
      </View>
      {flagLabel !== '' ? <Text dataSet={{ trackFlag: track.flag }}>{flagLabel}</Text> : null}
      <Text dataSet={{ trackDuration: '1' }}>{formatDuration(track.durationMs)}</Text>
    </View>
  );
}

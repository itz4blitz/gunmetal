import { useState } from 'react';
import type { KeyboardEvent, MouseEvent } from 'react';
import { Text, View } from 'react-native-web';
import type { DestinationMessages } from '../../messages/en/destinations.ts';
import { formatDuration } from '../format.ts';
import type { ShellTrack } from '../library-types.ts';
import { GoToArtistMenu } from './GoToArtistMenu.tsx';

export type TrackRowProps = {
  track: ShellTrack;
  messages: DestinationMessages;
  current?: boolean | undefined;
  artistKey?: string | undefined;
  onPlay: (albumId: string, trackId: string) => void;
  onPlayNext?: ((albumId: string, trackId: string) => void) | undefined;
  onAddToQueue?: ((albumId: string, trackId: string) => void) | undefined;
  onGoToAlbum?: ((albumId: string) => void) | undefined;
  onOpenArtist?: ((artistKey: string) => void) | undefined;
};

export function TrackRow({
  track,
  messages,
  current = false,
  artistKey,
  onPlay,
  onPlayNext,
  onAddToQueue,
  onGoToAlbum,
  onOpenArtist,
}: TrackRowProps) {
  const flagLabel =
    track.flag === 'unplayable'
      ? messages.trackFlagUnplayable
      : track.flag === 'damaged'
        ? messages.trackFlagDamaged
        : '';
  const [menuOpen, setMenuOpen] = useState(false);
  const openMenu = (event: { preventDefault: () => void; stopPropagation: () => void }) => {
    event.preventDefault();
    event.stopPropagation();
    setMenuOpen(true);
  };
  return (
    <View
      id={`track-row-${track.id}`}
      dataSet={{ trackRow: track.id, current: current ? '1' : '0' }}
      onContextMenu={
        onOpenArtist !== undefined && artistKey !== undefined
          ? (event) => {
              openMenu(event);
            }
          : undefined
      }
    >
      {current ? <View dataSet={{ nowPlaying: '1' }} /> : null}
      <View
        dataSet={{ trackPlay: '1' }}
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
        <Text dataSet={{ trackNumber: '1' }}>{`${track.number}`}</Text>
        <View dataSet={{ trackMeta: '1' }}>
          <Text dataSet={{ trackTitle: '1' }}>{track.title}</Text>
          <Text dataSet={{ trackArtist: '1' }}>{track.artistName}</Text>
        </View>
        {flagLabel !== '' ? <Text dataSet={{ trackFlag: track.flag }}>{flagLabel}</Text> : null}
        <Text dataSet={{ trackDuration: '1' }}>{formatDuration(track.durationMs)}</Text>
      </View>
      {onOpenArtist !== undefined && artistKey !== undefined ? (
        <>
          <View
            dataSet={{ itemMore: '1' }}
            accessibilityRole="button"
            accessibilityLabel={messages.moreActions}
            tabIndex={0}
            onClick={(event: MouseEvent<HTMLElement>) => {
              openMenu(event);
            }}
            onKeyDown={(event: KeyboardEvent<HTMLElement>) => {
              if (event.key === 'Enter' || event.key === ' ') {
                openMenu(event);
              }
            }}
          >
            <Text>{messages.moreActions}</Text>
          </View>
          <GoToArtistMenu
            open={menuOpen}
            artistKey={artistKey}
            messages={messages}
            onOpenArtist={onOpenArtist}
            onPlay={() => {
              onPlay(track.albumId, track.id);
            }}
            onPlayNext={() => {
              onPlayNext?.(track.albumId, track.id);
            }}
            onAddToQueue={() => {
              onAddToQueue?.(track.albumId, track.id);
            }}
            onGoToAlbum={() => {
              onGoToAlbum?.(track.albumId);
            }}
            onClose={() => {
              setMenuOpen(false);
            }}
          />
        </>
      ) : null}
    </View>
  );
}

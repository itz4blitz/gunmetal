import { useId, useState } from 'react';
import type { KeyboardEvent, MouseEvent } from 'react';
import { Text, View } from 'react-native-web';
import type { DestinationMessages } from '../../messages/en/destinations.ts';
import { formatDuration, staggerSlot } from '../format.ts';
import { anchorOf, type MenuPoint } from '../menu-anchor.ts';
import type { ShellTrack } from '../library-types.ts';
import { GoToArtistMenu } from './GoToArtistMenu.tsx';
import { Icon } from '../Icon.tsx';

export type TrackRowProps = {
  track: ShellTrack;
  messages: DestinationMessages;
  current?: boolean | undefined;
  artistKey?: string | undefined;
  /** Muted album column for library and search tables; album pages omit it. */
  albumTitle?: string | undefined;
  /** Hostile albums never render corpus text: safe catalogue labels replace it. */
  hostile?: boolean | undefined;
  /** Entrance choreography slot (capped); library tables stagger, search does not. */
  staggerIndex?: number | undefined;
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
  albumTitle,
  hostile = false,
  staggerIndex = 0,
  onPlay,
  onPlayNext,
  onAddToQueue,
  onGoToAlbum,
  onOpenArtist,
}: TrackRowProps) {
  const title = hostile ? messages.hostileAlbumLabel : track.title;
  const artist = hostile ? messages.hostileArtistLabel : track.artistName;
  const album = hostile ? messages.hostileAlbumLabel : albumTitle;
  const flagLabel =
    track.flag === 'unplayable'
      ? messages.trackFlagUnplayable
      : track.flag === 'damaged'
        ? messages.trackFlagDamaged
        : '';
  const menuId = useId();
  const [menuOpen, setMenuOpen] = useState(false);
  const [menuAt, setMenuAt] = useState<MenuPoint>({ x: 0, y: 0 });
  /* A track the server flagged as unplayable or damaged is not a play
     button: the row says why, and pressing it does not silently start a
     different track from the queue. */
  const playable = track.flag === 'ok';
  const openMenu = (event: {
    preventDefault: () => void;
    stopPropagation: () => void;
    clientX: number;
    clientY: number;
  }) => {
    event.preventDefault();
    event.stopPropagation();
    setMenuAt({ x: event.clientX, y: event.clientY });
    setMenuOpen(true);
  };
  /* The kebab is a toggle: a second press on it closes the menu it opened. */
  const toggleMenu = (event: { preventDefault: () => void; stopPropagation: () => void; currentTarget: Element }) => {
    event.preventDefault();
    event.stopPropagation();
    setMenuAt(anchorOf(event.currentTarget));
    setMenuOpen((open) => !open);
  };
  return (
    <View
      id={`track-row-${track.id}`}
      dataSet={{
        trackRow: track.id,
        current: current ? '1' : '0',
        hostile: hostile ? '1' : '0',
        flagged: flagLabel === '' ? '0' : '1',
        rowStagger: staggerSlot(staggerIndex),
      }}
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
        dataSet={{ trackPlay: '1', withAlbum: album === undefined ? '0' : '1' }}
        accessibilityRole={playable ? 'button' : undefined}
        accessibilityLabel={playable ? title : undefined}
        aria-disabled={playable ? undefined : true}
        tabIndex={playable ? 0 : undefined}
        onClick={
          playable
            ? () => {
                onPlay(track.albumId, track.id);
              }
            : undefined
        }
        onKeyDown={
          playable
            ? (event) => {
                if (event.key === 'Enter' || event.key === ' ') {
                  event.preventDefault();
                  onPlay(track.albumId, track.id);
                }
              }
            : undefined
        }
      >
        <Text dataSet={{ trackNumber: '1' }}>{`${track.number}`}</Text>
        <View dataSet={{ trackMeta: '1' }}>
          <Text dataSet={{ trackTitle: '1' }}>{title}</Text>
          <Text dataSet={{ trackArtist: '1' }}>{artist}</Text>
        </View>
        {album !== undefined ? <Text dataSet={{ trackAlbum: '1' }}>{album}</Text> : null}
        {flagLabel !== '' ? <Text dataSet={{ trackFlag: track.flag }}>{flagLabel}</Text> : null}
        <Text dataSet={{ trackDuration: '1' }}>{formatDuration(track.durationMs)}</Text>
      </View>
      {onOpenArtist !== undefined && artistKey !== undefined ? (
        <>
          <View
            dataSet={{ itemMore: '1' }}
            accessibilityRole="button"
            accessibilityLabel={messages.moreActions}
            aria-haspopup="menu"
            aria-expanded={menuOpen ? 'true' : 'false'}
            aria-controls={menuId}
            tabIndex={0}
            onClick={(event: MouseEvent<HTMLElement>) => {
              toggleMenu(event);
            }}
            onKeyDown={(event: KeyboardEvent<HTMLElement>) => {
              if (event.key === 'Enter' || event.key === ' ') {
                toggleMenu(event);
              }
            }}
          >
            <Icon name="more" size={18} />
          </View>
          <GoToArtistMenu
            open={menuOpen}
            menuId={menuId}
            artistKey={artistKey}
            messages={messages}
            at={menuAt}
            onOpenArtist={onOpenArtist}
            onPlay={
              playable
                ? () => {
                    onPlay(track.albumId, track.id);
                  }
                : undefined
            }
            onPlayNext={
              playable && onPlayNext !== undefined
                ? () => {
                    onPlayNext(track.albumId, track.id);
                  }
                : undefined
            }
            onAddToQueue={
              playable && onAddToQueue !== undefined
                ? () => {
                    onAddToQueue(track.albumId, track.id);
                  }
                : undefined
            }
            onGoToAlbum={
              onGoToAlbum !== undefined
                ? () => {
                    onGoToAlbum(track.albumId);
                  }
                : undefined
            }
            onClose={() => {
              setMenuOpen(false);
            }}
          />
        </>
      ) : null}
    </View>
  );
}

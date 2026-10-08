import { useId, useState } from 'react';
import type { KeyboardEvent, MouseEvent } from 'react';
import { Text, View } from 'react-native-web';
import type { DestinationMessages } from '../../messages/en/destinations.ts';
import { countNoun, formatDuration, staggerSlot } from '../format.ts';
import { anchorOf, type MenuPoint } from '../menu-anchor.ts';
import type { ShellAlbum } from '../library-types.ts';
import { CoverTile } from './CoverTile.tsx';
import { GoToArtistMenu } from './GoToArtistMenu.tsx';
import type { NearViewFactory } from './near-view.ts';
import { Icon } from '../Icon.tsx';

export type AlbumTileProps = {
  album: ShellAlbum;
  messages: DestinationMessages;
  onOpen: (albumId: string) => void;
  onPlay?: ((albumId: string) => void) | undefined;
  onPlayNext?: ((albumId: string) => void) | undefined;
  onAddToQueue?: ((albumId: string) => void) | undefined;
  onOpenArtist?: ((artistKey: string) => void) | undefined;
  staggerIndex?: number | undefined;
  /** This release is the one playing — brass where-you-are state on the title. */
  playing?: boolean | undefined;
  /** When wired, the cover art waits until the tile is near the viewport. */
  nearArt?: NearViewFactory | undefined;
};

function displayTitle(album: ShellAlbum, messages: DestinationMessages): string {
  if (album.hostile) {
    return messages.hostileAlbumLabel;
  }
  return album.title;
}

function displayArtist(album: ShellAlbum, messages: DestinationMessages): string {
  if (album.hostile) {
    return messages.hostileArtistLabel;
  }
  return album.artistName;
}

function albumDurationMs(album: ShellAlbum): number {
  return album.tracks.reduce((total, track) => total + track.durationMs, 0);
}

/**
 * Year only when the catalogue recorded one. An unknown year is omitted —
 * never printed as 0. Track count and total time are always stated.
 */
function albumFacts(album: ShellAlbum, messages: DestinationMessages): string {
  const count = countNoun(album.tracks.length, messages.trackCountOne, messages.trackCountLabel);
  const duration = formatDuration(albumDurationMs(album));
  if (album.year > 0) {
    return `${album.year} · ${count} · ${duration}`;
  }
  return `${count} · ${duration}`;
}

function activatePlay(
  albumId: string,
  onOpen: (albumId: string) => void,
  onPlay: ((albumId: string) => void) | undefined,
): void {
  if (onPlay !== undefined) {
    onPlay(albumId);
    return;
  }
  onOpen(albumId);
}

function activateOpenKey(event: KeyboardEvent<HTMLElement>, albumId: string, onOpen: (albumId: string) => void): void {
  if (event.key === 'Enter' || event.key === ' ') {
    event.preventDefault();
    onOpen(albumId);
  }
}

export function AlbumTile({
  album,
  messages,
  onOpen,
  onPlay,
  onPlayNext,
  onAddToQueue,
  onOpenArtist,
  staggerIndex = 0,
  playing = false,
  nearArt,
}: AlbumTileProps) {
  const title = displayTitle(album, messages);
  const artist = displayArtist(album, messages);
  const menuId = useId();
  const [menuOpen, setMenuOpen] = useState(false);
  const [menuAt, setMenuAt] = useState<MenuPoint>({ x: 0, y: 0 });
  /* A right-click opens at the pointer; the kebab hangs the menu off itself. */
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
      id={`album-tile-${album.id}`}
      dataSet={{
        albumTile: album.id,
        hostile: album.hostile ? '1' : '0',
        tilePlaying: playing ? '1' : '0',
        tileStagger: staggerSlot(staggerIndex),
      }}
      onContextMenu={
        onOpenArtist === undefined
          ? undefined
          : (event) => {
              openMenu(event);
            }
      }
    >
      <View
        dataSet={{ albumArt: '1' }}
        onClick={() => {
          onOpen(album.id);
        }}
      >
        <CoverTile
          tone={album.coverTone}
          label={title}
          size="grid"
          coverId={`cover-grid-${album.id}`}
          artUrl={album.coverUrl}
          deferArt={nearArt}
        />
        {/* 2026 pattern: the hover scrim the controls sit on. Decorative —
            the play hex and kebab above it are the real controls. */}
        <View dataSet={{ artScrim: '1' }} aria-hidden="true" />
        <View
          dataSet={{ albumPlay: '1' }}
          accessibilityRole="button"
          accessibilityLabel={messages.playAlbum}
          tabIndex={0}
          onClick={(event: MouseEvent<HTMLElement>) => {
            event.stopPropagation();
            activatePlay(album.id, onOpen, onPlay);
          }}
          onKeyDown={(event: KeyboardEvent<HTMLElement>) => {
            if (event.key === 'Enter' || event.key === ' ') {
              event.preventDefault();
              event.stopPropagation();
              activatePlay(album.id, onOpen, onPlay);
            }
          }}
        />
        {onOpenArtist === undefined ? null : (
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
        )}
      </View>
      <View
        dataSet={{ albumOpen: '1' }}
        accessibilityRole="button"
        accessibilityLabel={title}
        tabIndex={0}
        onClick={() => {
          onOpen(album.id);
        }}
        onKeyDown={(event) => {
          activateOpenKey(event, album.id, onOpen);
        }}
      >
        <Text dataSet={{ albumTitle: '1' }}>{title}</Text>
        <Text dataSet={{ albumArtist: '1' }}>{artist}</Text>
        <Text dataSet={{ albumFacts: '1' }}>{albumFacts(album, messages)}</Text>
      </View>
      {onOpenArtist === undefined ? null : (
        <GoToArtistMenu
          open={menuOpen}
          menuId={menuId}
          artistKey={album.artistKey}
          messages={messages}
          at={menuAt}
          onOpenArtist={onOpenArtist}
          onPlay={() => {
            activatePlay(album.id, onOpen, onPlay);
          }}
          onPlayNext={() => {
            onPlayNext?.(album.id);
          }}
          onAddToQueue={() => {
            onAddToQueue?.(album.id);
          }}
          onGoToAlbum={() => {
            onOpen(album.id);
          }}
          onClose={() => {
            setMenuOpen(false);
          }}
        />
      )}
    </View>
  );
}

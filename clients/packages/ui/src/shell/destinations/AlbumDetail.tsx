import { useCallback, useId, useState } from 'react';
import type { MouseEvent } from 'react';
import { Text, View } from 'react-native-web';
import type { DestinationMessages } from '../../messages/en/destinations.ts';
import type { ShellAlbum, ShellTrack } from '../library-types.ts';
import { formatDuration } from '../format.ts';
import { LyricsPane } from '../LyricsPane.tsx';
import { noLyrics, type LyricsResolver } from '../content.ts';
import { Icon } from '../Icon.tsx';
import { useMenuDismiss } from '../menu-dismiss.ts';
import { AlbumTile } from './AlbumTile.tsx';
import { CoverTile } from './CoverTile.tsx';
import { TrackRow } from './TrackRow.tsx';

function albumLyricsTrack(album: ShellAlbum, currentTrackId?: string): ShellTrack | undefined {
  const current = album.tracks.find((track) => track.id === currentTrackId);
  if (current !== undefined && (current.lyricsKind === 'plain' || current.lyricsKind === 'synced')) {
    return current;
  }
  return album.tracks.find((track) => track.lyricsKind === 'plain' || track.lyricsKind === 'synced');
}

/** The header's year · track count · total time line is computed from the tracks, never hard-coded. */
function albumTotalDuration(album: ShellAlbum): number {
  return album.tracks.reduce((total, track) => total + track.durationMs, 0);
}

function discHeadingLabel(disc: { index: number; title: string }, messages: DestinationMessages): string {
  return disc.title === '' ? `${messages.discsHeading} ${disc.index}` : disc.title;
}

/** Same-origin artwork as a CSS background layer; empty URLs paint nothing. */
function artLayer(url: string | undefined): { backgroundImage: string } | undefined {
  return url === undefined || url === '' ? undefined : { backgroundImage: `url("${url}")` };
}

/** The album kebab's actions are listed only when the shell wired them —
 * an item that opens and does nothing would lie (design-language §3.5). */
type AlbumMenuAction = { id: 'play' | 'play-next' | 'add-to-queue' | 'go-to-artist'; label: string; run: () => void };

function albumMenuActions(
  album: ShellAlbum,
  messages: DestinationMessages,
  handlers: {
    onPlayAlbum: (albumId: string) => void;
    onPlayNextAlbum?: ((albumId: string) => void) | undefined;
    onAddAlbumToQueue?: ((albumId: string) => void) | undefined;
    onOpenArtist?: ((artistKey: string) => void) | undefined;
  },
): AlbumMenuAction[] {
  const actions: AlbumMenuAction[] = [
    { id: 'play', label: messages.playAlbum, run: () => handlers.onPlayAlbum(album.id) },
  ];
  const { onPlayNextAlbum, onAddAlbumToQueue, onOpenArtist } = handlers;
  if (onPlayNextAlbum !== undefined) {
    actions.push({ id: 'play-next', label: messages.playNext, run: () => onPlayNextAlbum(album.id) });
  }
  if (onAddAlbumToQueue !== undefined) {
    actions.push({ id: 'add-to-queue', label: messages.addToQueue, run: () => onAddAlbumToQueue(album.id) });
  }
  if (onOpenArtist !== undefined) {
    actions.push({ id: 'go-to-artist', label: messages.goToArtist, run: () => onOpenArtist(album.artistKey) });
  }
  return actions;
}

export type AlbumDetailProps = {
  lyricsFor?: LyricsResolver | undefined;
  album: ShellAlbum | undefined;
  messages: DestinationMessages;
  currentTrackId?: string | undefined;
  onBack: () => void;
  onPlayAlbum: (albumId: string) => void;
  onPlayTrack: (albumId: string, trackId: string) => void;
  onOpenArtist?: ((artistKey: string) => void) | undefined;
  onShuffleAlbum?: ((albumId: string) => void) | undefined;
  onPlayNextAlbum?: ((albumId: string) => void) | undefined;
  onAddAlbumToQueue?: ((albumId: string) => void) | undefined;
  onPlayNextTrack?: ((albumId: string, trackId: string) => void) | undefined;
  onAddTrackToQueue?: ((albumId: string, trackId: string) => void) | undefined;
  /** The artist's other releases and how to open one; omitted when there are none. */
  moreBy?: AlbumMoreBy | undefined;
};

export type AlbumMoreBy = {
  albums: readonly ShellAlbum[];
  onOpenAlbum: (albumId: string) => void;
};

/**
 * A hostile album never renders corpus text, not even through a row: these
 * rows carry a safe catalogue label, the real duration, and nothing else.
 */
function HostileTrackRow({
  track,
  current,
  label,
  onPlay,
}: {
  track: ShellTrack;
  current: boolean;
  label: string;
  onPlay: (albumId: string, trackId: string) => void;
}) {
  return (
    <View id={`track-row-${track.id}`} dataSet={{ albumRow: '1', hostileRow: '1', current: current ? '1' : '0' }}>
      {current ? <View dataSet={{ nowPlaying: '1' }} /> : null}
      <View
        dataSet={{ hostileRowPlay: '1' }}
        accessibilityRole="button"
        accessibilityLabel={`${label} ${track.number}`}
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
        <Text dataSet={{ hostileRowLabel: '1' }}>{label}</Text>
        <Text dataSet={{ trackDuration: '1' }}>{formatDuration(track.durationMs)}</Text>
      </View>
    </View>
  );
}

export function AlbumDetail({
  lyricsFor = () => noLyrics,
  album,
  messages,
  currentTrackId,
  onBack,
  onPlayAlbum,
  onPlayTrack,
  onOpenArtist,
  onShuffleAlbum,
  onPlayNextAlbum,
  onAddAlbumToQueue,
  onPlayNextTrack,
  onAddTrackToQueue,
  moreBy,
}: AlbumDetailProps) {
  const [lyricsOpen, setLyricsOpen] = useState(false);
  const [moreOpen, setMoreOpen] = useState(false);
  const moreMenuId = useId();
  const closeMore = useCallback(() => {
    setMoreOpen(false);
  }, []);
  useMenuDismiss(moreOpen, moreMenuId, closeMore);

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
          <Icon name="back" size={18} />
          <Text>{messages.backToLibrary}</Text>
        </View>
      </View>
    );
  }

  const title = album.hostile ? messages.hostileAlbumLabel : album.title;
  const artist = album.hostile ? messages.hostileArtistLabel : album.artistName;
  const lyricsTrack = albumLyricsTrack(album, currentTrackId);
  const shuffleWired = onShuffleAlbum !== undefined;
  const menuActions = albumMenuActions(album, messages, {
    onPlayAlbum,
    onPlayNextAlbum,
    onAddAlbumToQueue,
    onOpenArtist,
  });
  const toggleMore = () => {
    setMoreOpen((open) => !open);
  };

  const renderTrackRow = (track: ShellTrack) => (
    <View
      key={track.id}
      dataSet={{
        albumRow: '1',
        guest: track.artistName === album.artistName ? '0' : '1',
      }}
    >
      <TrackRow
        track={track}
        messages={messages}
        current={track.id === currentTrackId}
        artistKey={album.artistKey}
        onPlay={onPlayTrack}
        onPlayNext={onPlayNextTrack}
        onAddToQueue={onAddTrackToQueue}
        onOpenArtist={onOpenArtist}
      />
    </View>
  );

  return (
    <View
      id="destination-album"
      dataSet={{
        albumId: album.id,
        hostile: album.hostile ? '1' : '0',
        artTone: album.coverTone,
      }}
    >
      {/* Hero bloom: the artwork, blurred and faded into the canvas, sits
          behind the header; the scrim keeps the display type legible.
          Both are decoration — aria-hidden, pointer-events none. */}
      <View dataSet={{ albumBloom: '1' }} aria-hidden={true} style={artLayer(album.coverUrl)} />
      <View dataSet={{ albumScrim: '1' }} aria-hidden={true} />
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
        <Icon name="back" size={18} />
        <Text>{messages.backToLibrary}</Text>
      </View>
      <View dataSet={{ albumHeader: '1', albumHeaderLarge: '1', albumHeaderBleed: '1' }}>
        <CoverTile
          tone={album.coverTone}
          label={title}
          size="detail"
          coverId={`cover-detail-${album.id}`}
          artUrl={album.coverUrl}
        />
        <View dataSet={{ albumHeaderText: '1' }}>
          <Text dataSet={{ detailEyebrow: '1' }}>{messages.albumEyebrow}</Text>
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
          <View dataSet={{ albumMeta: '1' }}>
            <Text id="album-year" dataSet={{ albumYear: '1' }}>
              {`${album.year}`}
            </Text>
            <Text dataSet={{ metaSeparator: '1' }}>·</Text>
            <Text id="album-track-count" dataSet={{ albumTrackCount: '1' }}>
              {`${album.tracks.length} ${messages.trackCountLabel}`}
            </Text>
            <Text dataSet={{ metaSeparator: '1' }}>·</Text>
            <Text id="album-duration-total" dataSet={{ albumDuration: '1' }}>
              {formatDuration(albumTotalDuration(album))}
            </Text>
          </View>
          {album.license === undefined ? null : (
            <Text
              id="album-license"
              dataSet={{ albumLicense: album.license.spdx }}
            >{`${messages.licenseLabel} ${album.license.spdx} · ${album.license.attribution} · ${album.license.source}`}</Text>
          )}
        </View>
      </View>
      {/* Sticky action rail: the page's Play · Shuffle · Lyrics · More row.
          It is the header's only action row — compact hexes that pin to the
          content's top edge while the track list scrolls beneath them. */}
      <View dataSet={{ albumRail: '1', albumActions: '1' }}>
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
        <View dataSet={{ shuffleWrap: '1', wired: shuffleWired ? '1' : '0' }}>
          <View
            id="album-shuffle"
            dataSet={{ hexFace: '1' }}
            accessibilityRole="button"
            accessibilityLabel={messages.shuffle}
            aria-disabled={shuffleWired ? undefined : true}
            tabIndex={0}
            onClick={() => {
              if (shuffleWired) {
                onShuffleAlbum(album.id);
              }
            }}
            onKeyDown={(event) => {
              if (shuffleWired && (event.key === 'Enter' || event.key === ' ')) {
                event.preventDefault();
                onShuffleAlbum(album.id);
              }
            }}
          >
            <Icon name="shuffle" />
          </View>
          <Text dataSet={{ controlHint: '1' }}>{shuffleWired ? messages.shuffle : messages.shuffleUnavailable}</Text>
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
            <Icon name="lyrics" size={18} />
            <Text>{messages.lyrics}</Text>
          </View>
        )}
        <View dataSet={{ kebabWrap: '1' }}>
          <View
            id="album-more"
            dataSet={{ albumMore: '1' }}
            accessibilityRole="button"
            accessibilityLabel={messages.moreActions}
            aria-haspopup="menu"
            aria-expanded={moreOpen ? 'true' : 'false'}
            aria-controls={moreMenuId}
            tabIndex={0}
            onClick={toggleMore}
            onKeyDown={(event) => {
              if (event.key === 'Enter' || event.key === ' ') {
                event.preventDefault();
                toggleMore();
              }
            }}
          >
            <Icon name="more" />
          </View>
          {moreOpen ? (
            <View
              dataSet={{ albumMenu: '1', contextMenu: '1', menuId: moreMenuId }}
              accessibilityRole="menu"
              accessibilityLabel={messages.contextMenu}
            >
              {menuActions.map((action) => (
                <View
                  key={action.id}
                  dataSet={{ menuItem: action.id }}
                  accessibilityRole="menuitem"
                  accessibilityLabel={action.label}
                  tabIndex={0}
                  onClick={(event: MouseEvent<HTMLElement>) => {
                    event.stopPropagation();
                    action.run();
                    setMoreOpen(false);
                  }}
                  onKeyDown={(event) => {
                    if (event.key === 'Enter' || event.key === ' ') {
                      event.preventDefault();
                      event.stopPropagation();
                      action.run();
                      setMoreOpen(false);
                    }
                  }}
                >
                  <Text dataSet={{ menuLabel: '1' }}>{action.label}</Text>
                </View>
              ))}
            </View>
          ) : null}
        </View>
      </View>
      {lyricsTrack === undefined ? null : (
        <LyricsPane
          empty={lyricsTrack.lyricsKind === 'none'}
          id="album-lyrics"
          label={messages.lyricsRegion}
          lines={lyricsFor(lyricsTrack.id, lyricsTrack.lyricsKind)}
          synced={lyricsTrack.lyricsKind === 'synced'}
          open={lyricsOpen}
        />
      )}
      {album.discs.length > 1 ? (
        album.discs.map((disc) => {
          const discTracks = album.tracks.filter((track) => track.discIndex === disc.index);
          const firstTrack = discTracks[0];
          return (
            <View key={disc.index} dataSet={{ discBlock: `${disc.index}` }}>
              <View dataSet={{ discHeaderRow: '1' }}>
                <Text accessibilityRole="header" dataSet={{ discHeader: '1', type: 'title2' }}>
                  {discHeadingLabel(disc, messages)}
                </Text>
                {firstTrack === undefined ? null : (
                  <View
                    dataSet={{ discPlay: '1' }}
                    accessibilityRole="button"
                    accessibilityLabel={
                      disc.title === '' ? `${messages.playDisc} ${disc.index}` : `${messages.playDisc} · ${disc.title}`
                    }
                    tabIndex={0}
                    onClick={() => {
                      onPlayTrack(album.id, firstTrack.id);
                    }}
                    onKeyDown={(event) => {
                      if (event.key === 'Enter' || event.key === ' ') {
                        event.preventDefault();
                        onPlayTrack(album.id, firstTrack.id);
                      }
                    }}
                  >
                    <Text dataSet={{ discPlayLabel: '1' }}>{messages.playDisc}</Text>
                  </View>
                )}
              </View>
              <View dataSet={{ trackTableHead: '1' }} aria-hidden={true}>
                <Text dataSet={{ trackTableNumber: '1' }}>{messages.columnNumber}</Text>
                <Text dataSet={{ trackTableTitle: '1' }}>{messages.columnTitle}</Text>
                <View dataSet={{ trackTableTime: '1' }}>
                  <Icon name="clock" size={16} />
                </View>
              </View>
              {album.hostile
                ? discTracks.map((track) => (
                    <HostileTrackRow
                      key={track.id}
                      track={track}
                      current={track.id === currentTrackId}
                      label={messages.hostileTrackHidden}
                      onPlay={onPlayTrack}
                    />
                  ))
                : discTracks.map(renderTrackRow)}
            </View>
          );
        })
      ) : (
        <View dataSet={{ discBlock: '1' }}>
          <Text accessibilityRole="header" dataSet={{ discHeader: '1', type: 'title2' }}>
            {messages.tracksHeading}
          </Text>
          <View dataSet={{ trackTableHead: '1' }} aria-hidden={true}>
            <Text dataSet={{ trackTableNumber: '1' }}>{messages.columnNumber}</Text>
            <Text dataSet={{ trackTableTitle: '1' }}>{messages.columnTitle}</Text>
            <View dataSet={{ trackTableTime: '1' }}>
              <Icon name="clock" size={16} />
            </View>
          </View>
          {album.hostile
            ? album.tracks.map((track) => (
                <HostileTrackRow
                  key={track.id}
                  track={track}
                  current={track.id === currentTrackId}
                  label={messages.hostileTrackHidden}
                  onPlay={onPlayTrack}
                />
              ))
            : album.tracks.map(renderTrackRow)}
        </View>
      )}
      {moreBy === undefined ? null : (
        <View id="album-more-by" dataSet={{ albumMoreBy: '1' }}>
          <Text accessibilityRole="header" dataSet={{ sectionHeading: '1', type: 'title2' }}>
            {`${messages.moreByArtist} ${artist}`}
          </Text>
          <View dataSet={{ albumGrid: '1', albumMoreByGrid: '1' }}>
            {moreBy.albums.map((other, index) => (
              <AlbumTile
                key={other.id}
                album={other}
                messages={messages}
                staggerIndex={index}
                onOpen={moreBy.onOpenAlbum}
                onPlay={onPlayAlbum}
                onPlayNext={onPlayNextAlbum}
                onAddToQueue={onAddAlbumToQueue}
                onOpenArtist={onOpenArtist}
              />
            ))}
          </View>
        </View>
      )}
    </View>
  );
}

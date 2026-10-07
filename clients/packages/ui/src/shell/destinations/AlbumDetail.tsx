import { useState } from 'react';
import { Text, View } from 'react-native-web';
import type { DestinationMessages } from '../../messages/en/destinations.ts';
import type { ShellAlbum, ShellTrack } from '../library-types.ts';
import { formatDuration } from '../format.ts';
import { LyricsPane } from '../LyricsPane.tsx';
import { noLyrics, type LyricsResolver } from '../content.ts';
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
  const shuffleWired = onShuffleAlbum !== undefined;

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
          artUrl={album.coverUrl}
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
          <View dataSet={{ albumActions: '1' }}>
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
                <View dataSet={{ shuffleGlyph: '1' }}>
                  <View dataSet={{ shuffleArm: 'a' }} />
                  <View dataSet={{ shuffleArm: 'b' }} />
                </View>
              </View>
              <Text dataSet={{ controlHint: '1' }}>
                {shuffleWired ? messages.shuffle : messages.shuffleUnavailable}
              </Text>
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
    </View>
  );
}

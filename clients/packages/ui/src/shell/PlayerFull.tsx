import { useEffect, useState } from 'react';
import { Text, View } from 'react-native-web';
import { demoLyricsLines, demoLyricsVerse } from '../../../fake-server/src/lyrics.ts';
import type { ShellMessages } from '../messages/en/shell.ts';
import { CoverTile } from './destinations/CoverTile.tsx';
import { formatDuration } from './format.ts';
import { LyricsPane } from './LyricsPane.tsx';
import type { PlaybackSnapshot, QueueLine } from './playback.ts';

export type PlayerPlacement = 'overlay' | 'pane';

export type PlayerFullProps = {
  messages: ShellMessages;
  playback: PlaybackSnapshot;
  open: boolean;
  placement?: PlayerPlacement;
  albumTitle?: string;
  onClose: () => void;
  onPlayPause?: () => void;
  onPrevious?: () => void;
  onNext?: () => void;
  onToggleQueue?: () => void;
};

function nextQueueLine(playback: PlaybackSnapshot): QueueLine | undefined {
  const index = playback.queue.findIndex((line) => line.trackId === playback.trackId);
  if (index < 0) {
    return undefined;
  }
  return playback.queue[index + 1];
}

export function PlayerFull({
  messages,
  playback,
  open,
  placement = 'overlay',
  albumTitle,
  onClose,
  onPlayPause,
  onPrevious,
  onNext,
  onToggleQueue,
}: PlayerFullProps) {
  const [lyricsOpen, setLyricsOpen] = useState(false);
  const canLyrics = playback.lyricsKind === 'plain' || playback.lyricsKind === 'synced';
  useEffect(() => {
    if (!canLyrics) {
      setLyricsOpen(false);
    }
  }, [canLyrics]);
  useEffect(() => {
    if (!open) {
      return;
    }
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        onClose();
      }
    };
    globalThis.addEventListener('keydown', onKey);
    return () => {
      globalThis.removeEventListener('keydown', onKey);
    };
  }, [open, onClose]);

  if (playback.trackId === undefined || !open) {
    return null;
  }

  const progress =
    playback.durationMs > 0 ? Math.min(1, playback.positionMs / playback.durationMs) : 0;
  const remainingMs =
    playback.durationMs > 0 ? Math.max(0, playback.durationMs - playback.positionMs) : 0;
  const upNext = nextQueueLine(playback);
  const fromLabel =
    albumTitle !== undefined && albumTitle !== '' ? `${messages.playingFrom} ${albumTitle}` : undefined;

  return (
    <>
      {placement === 'overlay' ? (
        <View id="player-full-scrim" dataSet={{ open: '1' }} onClick={onClose} />
      ) : null}
      <View
        id="player-full"
        accessibilityRole="dialog"
        accessibilityLabel={messages.playerFullRegion}
        dataSet={{ open: '1', placement }}
      >
        <View
          id="player-full-close"
          accessibilityRole="button"
          accessibilityLabel={messages.playerClose}
          tabIndex={0}
          onClick={onClose}
          onKeyDown={(event) => {
            if (event.key === 'Enter' || event.key === ' ') {
              event.preventDefault();
              onClose();
            }
          }}
        >
          <Text>{messages.playerClose}</Text>
        </View>
        {fromLabel === undefined ? null : <Text id="player-full-from">{fromLabel}</Text>}
        <View id="player-full-art">
          <CoverTile
            tone={playback.coverTone}
            label={playback.title}
            size="full"
            coverId={`cover-full-${playback.trackId}`}
          />
        </View>
        <Text id="player-full-title">{playback.title}</Text>
        <Text id="player-full-artist">{playback.artistName}</Text>
        <View
          id="player-full-progress"
          accessibilityRole="progressbar"
          accessibilityLabel={messages.progress}
          dataSet={{ progress: `${Math.round(progress * 100)}` }}
        >
          <View id="player-full-progress-track">
            <View
              id="player-full-progress-fill"
              dataSet={{ fill: `${Math.round(progress * 100)}` }}
              style={{ width: `${Math.round(progress * 100)}%` }}
            />
          </View>
          <View id="player-full-time">
            <Text id="player-full-elapsed">{formatDuration(playback.positionMs)}</Text>
            <Text id="player-full-remaining">{formatDuration(remainingMs)}</Text>
          </View>
        </View>
        <View id="player-full-transport">
          <FullControl
            id="player-full-prev"
            label={messages.previous}
            onPress={onPrevious}
          />
          <FullControl
            id="player-full-play"
            label={playback.playing ? messages.pause : messages.play}
            onPress={onPlayPause}
            primary
            playing={playback.playing}
          />
          <FullControl id="player-full-next" label={messages.next} onPress={onNext} />
        </View>
        <View id="player-full-footer">
          <View id="player-full-device" dataSet={{ deviceSlot: 'empty' }} />
          <View
            id="player-full-lyrics-toggle"
            accessibilityRole="button"
            accessibilityLabel={messages.lyrics}
            tabIndex={0}
            dataSet={{
              lyricsToggle: lyricsOpen ? '1' : '0',
              lyricsAvailable: canLyrics ? '1' : '0',
            }}
            onClick={() => {
              if (canLyrics) {
                setLyricsOpen((open) => !open);
              }
            }}
            onKeyDown={(event) => {
              if (!canLyrics) {
                return;
              }
              if (event.key === 'Enter' || event.key === ' ') {
                event.preventDefault();
                setLyricsOpen((open) => !open);
              }
            }}
          >
            <Text>{messages.lyrics}</Text>
          </View>
          <View
            id="player-full-queue"
            accessibilityRole="button"
            accessibilityLabel={messages.queue}
            tabIndex={0}
            onClick={() => {
              onToggleQueue?.();
            }}
            onKeyDown={(event) => {
              if (event.key === 'Enter' || event.key === ' ') {
                event.preventDefault();
                onToggleQueue?.();
              }
            }}
          >
            <Text>{messages.queue}</Text>
          </View>
        </View>
        {upNext === undefined ? null : (
          <View id="player-full-up-next">
            <Text id="player-full-up-next-label">{messages.queueHeading}</Text>
            <Text id="player-full-up-next-title">{upNext.title}</Text>
            <Text id="player-full-up-next-artist">{upNext.artistName}</Text>
          </View>
        )}
        {canLyrics ? (
          <LyricsPane
            id="player-full-lyrics"
            label={messages.lyrics}
            lines={demoLyricsLines(demoLyricsVerse(playback.trackId, playback.lyricsKind))}
            synced={playback.lyricsKind === 'synced'}
            open={lyricsOpen}
          />
        ) : (
          <Text id="player-full-lyrics-unavailable">{messages.lyricsUnavailable}</Text>
        )}
      </View>
    </>
  );
}

type FullControlProps = {
  id: string;
  label: string;
  onPress?: () => void;
  primary?: boolean;
  playing?: boolean;
};

function FullControl({
  id,
  label,
  onPress,
  primary = false,
  playing = false,
}: FullControlProps) {
  return (
    <View
      id={id}
      dataSet={{
        playerControl: primary ? 'primary' : 'plain',
        playing: playing ? '1' : '0',
      }}
      accessibilityRole="button"
      accessibilityLabel={label}
      tabIndex={0}
      onClick={() => {
        onPress?.();
      }}
      onKeyDown={(event) => {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          onPress?.();
        }
      }}
    >
      <Text dataSet={{ controlLabel: '1' }}>{label}</Text>
    </View>
  );
}

import { useEffect, useState } from 'react';
import { Text, View } from 'react-native-web';
import { demoLyricsLines, demoLyricsVerse } from '../../../fake-server/src/lyrics.ts';
import type { ShellMessages } from '../messages/en/shell.ts';
import { CoverTile } from './destinations/CoverTile.tsx';
import { formatDuration } from './format.ts';
import { LyricsPane } from './LyricsPane.tsx';
import type { PlaybackSnapshot } from './playback.ts';

export type PlayerFullProps = {
  messages: ShellMessages;
  playback: PlaybackSnapshot;
  open: boolean;
  onClose: () => void;
  onPlayPause?: () => void;
  onPrevious?: () => void;
  onNext?: () => void;
};

export function PlayerFull({
  messages,
  playback,
  open,
  onClose,
  onPlayPause,
  onPrevious,
  onNext,
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

  return (
    <>
      <View id="player-full-scrim" dataSet={{ open: '1' }} onClick={onClose} />
      <View
        id="player-full"
        accessibilityRole="dialog"
        accessibilityLabel={messages.playerFullRegion}
        dataSet={{ open: '1' }}
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
          <Text id="player-full-time">
            {`${formatDuration(playback.positionMs)} / ${formatDuration(playback.durationMs)}`}
          </Text>
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
        {canLyrics ? (
          <View
            id="player-full-lyrics-toggle"
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
        ) : null}
        <LyricsPane
          id="player-full-lyrics"
          label={messages.lyrics}
          lines={demoLyricsLines(demoLyricsVerse(playback.trackId, playback.lyricsKind))}
          synced={playback.lyricsKind === 'synced'}
          open={lyricsOpen && canLyrics}
        />
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

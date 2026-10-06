import { useEffect } from 'react';
import { Text, View } from 'react-native-web';
import type { ShellMessages } from '../messages/en/shell.ts';
import { CoverTile } from './destinations/CoverTile.tsx';
import { formatDuration } from './format.ts';
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

  if (playback.trackId === undefined) {
    return null;
  }

  const progress =
    playback.durationMs > 0 ? Math.min(1, playback.positionMs / playback.durationMs) : 0;
  const openFlag = open ? '1' : '0';

  return (
    <>
      <View
        id="player-full-scrim"
        dataSet={{ open: openFlag }}
        accessibilityElementsHidden={!open}
        onClick={onClose}
      />
      <View
        id="player-full"
        accessibilityRole="dialog"
        accessibilityLabel={messages.playerFullRegion}
        accessibilityElementsHidden={!open}
        dataSet={{ open: openFlag }}
      >
        <View
          id="player-full-close"
          accessibilityRole="button"
          accessibilityLabel={messages.playerClose}
          tabIndex={open ? 0 : -1}
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
            tabbable={open}
          />
          <FullControl
            id="player-full-play"
            label={playback.playing ? messages.pause : messages.play}
            onPress={onPlayPause}
            tabbable={open}
            primary
            playing={playback.playing}
          />
          <FullControl
            id="player-full-next"
            label={messages.next}
            onPress={onNext}
            tabbable={open}
          />
        </View>
      </View>
    </>
  );
}

type FullControlProps = {
  id: string;
  label: string;
  onPress?: () => void;
  tabbable: boolean;
  primary?: boolean;
  playing?: boolean;
};

function FullControl({
  id,
  label,
  onPress,
  tabbable,
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
      tabIndex={tabbable ? 0 : -1}
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

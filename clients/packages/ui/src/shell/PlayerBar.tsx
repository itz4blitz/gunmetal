import { Text, View } from 'react-native-web';
import type { ShellMessages } from '../messages/en/shell.ts';
import { CoverTile } from './destinations/CoverTile.tsx';
import { formatDuration } from './format.ts';
import type { PlaybackSnapshot } from './playback.ts';

export type PlayerBarProps = {
  messages: ShellMessages;
  playback: PlaybackSnapshot;
  onPlayPause?: () => void;
  onPrevious?: () => void;
  onNext?: () => void;
  onToggleQueue?: () => void;
};

export function PlayerBar({
  messages,
  playback,
  onPlayPause,
  onPrevious,
  onNext,
  onToggleQueue,
}: PlayerBarProps) {
  const empty = playback.trackId === undefined;
  const progress =
    playback.durationMs > 0 ? Math.min(1, playback.positionMs / playback.durationMs) : 0;

  return (
    <View id="player-bar" accessibilityRole="region" accessibilityLabel={messages.playerRegion}>
      {empty ? (
        <View id="player-now" dataSet={{ empty: '1' }}>
          <Text id="player-empty">{messages.playerEmpty}</Text>
        </View>
      ) : (
        <View id="player-now" dataSet={{ playing: playback.playing ? '1' : '0' }}>
          <CoverTile tone={playback.coverTone} label={playback.title} size="bar" />
          <View id="player-meta">
            <Text id="player-title">{playback.title}</Text>
            <Text id="player-artist">{playback.artistName}</Text>
          </View>
        </View>
      )}
      <View id="player-transport">
        <ControlButton
          id="player-prev"
          label={messages.previous}
          onPress={onPrevious}
          disabled={empty}
        />
        <ControlButton
          id="shell-play"
          label={playback.playing ? messages.pause : messages.play}
          onPress={onPlayPause}
          disabled={empty}
          primary
          playing={playback.playing}
        />
        <ControlButton id="player-next" label={messages.next} onPress={onNext} disabled={empty} />
      </View>
      <View
        id="player-progress"
        accessibilityRole="progressbar"
        accessibilityLabel={messages.progress}
        dataSet={{ progress: `${Math.round(progress * 100)}` }}
      >
        <View id="player-progress-track">
          <View
            id="player-progress-fill"
            dataSet={{ fill: `${Math.round(progress * 100)}` }}
            style={{ width: `${Math.round(progress * 100)}%` }}
          />
        </View>
        <Text id="player-time">
          {empty
            ? '0:00 / 0:00'
            : `${formatDuration(playback.positionMs)} / ${formatDuration(playback.durationMs)}`}
        </Text>
      </View>
      <View id="player-actions">
        <ControlButton
          id="player-queue"
          label={messages.queue}
          onPress={onToggleQueue}
          disabled={empty}
        />
      </View>
    </View>
  );
}

type ControlButtonProps = {
  id: string;
  label: string;
  onPress?: () => void;
  disabled?: boolean;
  primary?: boolean;
  playing?: boolean;
};

function ControlButton({
  id,
  label,
  onPress,
  disabled = false,
  primary = false,
  playing = false,
}: ControlButtonProps) {
  return (
    <View
      id={id}
      dataSet={{
        playerControl: primary ? 'primary' : 'plain',
        disabled: disabled ? '1' : '0',
        playing: playing ? '1' : '0',
      }}
      accessibilityRole="button"
      accessibilityLabel={label}
      accessibilityState={{ disabled }}
      tabIndex={disabled ? -1 : 0}
      onClick={() => {
        if (!disabled) {
          onPress?.();
        }
      }}
      onKeyDown={(event) => {
        if (disabled) {
          return;
        }
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

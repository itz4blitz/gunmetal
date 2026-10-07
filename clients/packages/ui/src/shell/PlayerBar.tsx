import { Text, View } from 'react-native-web';
import type { ShellMessages } from '../messages/en/shell.ts';
import { CoverTile } from './destinations/CoverTile.tsx';
import { formatDuration } from './format.ts';
import type { PlayerSnapshot } from '../../../ports/src/provisional/player.ts';

export type PlayerBarProps = {
  messages: ShellMessages;
  playback: PlayerSnapshot;
  albumTitle?: string | undefined;
  compact?: boolean | undefined;
  volume?: number | undefined;
  onVolume?: ((volume: number) => void) | undefined;
  onSeek?: ((positionMs: number) => void) | undefined;
  onPlayPause?: (() => void) | undefined;
  onPrevious?: (() => void) | undefined;
  onNext?: (() => void) | undefined;
  onToggleQueue?: (() => void) | undefined;
  onOpenFull?: (() => void) | undefined;
};

export function PlayerBar({
  messages,
  playback,
  albumTitle,
  compact = false,
  volume,
  onVolume,
  onSeek,
  onPlayPause,
  onPrevious,
  onNext,
  onToggleQueue,
  onOpenFull,
}: PlayerBarProps) {
  const empty = playback.trackId === undefined;
  const progress = playback.durationMs > 0 ? Math.min(1, playback.positionMs / playback.durationMs) : 0;

  const openFull = () => {
    onOpenFull?.();
  };

  const seekFromEvent = (event: { currentTarget: HTMLElement; clientX: number }) => {
    if (onSeek === undefined || empty || playback.durationMs <= 0) {
      return;
    }
    const rect = event.currentTarget.getBoundingClientRect();
    if (rect.width <= 0) {
      return;
    }
    const fraction = Math.max(0, Math.min(1, (event.clientX - rect.left) / rect.width));
    onSeek(Math.round(fraction * playback.durationMs));
  };

  const seekByKeyboard = (event: { key: string; preventDefault: () => void }) => {
    if (onSeek === undefined || empty || playback.durationMs <= 0) {
      return;
    }
    const step = 5000;
    if (event.key === 'ArrowLeft') {
      event.preventDefault();
      onSeek(Math.max(0, playback.positionMs - step));
      return;
    }
    if (event.key === 'ArrowRight') {
      event.preventDefault();
      onSeek(Math.min(playback.durationMs, playback.positionMs + step));
    }
  };

  return (
    <View id="player-bar" accessibilityRole="region" accessibilityLabel={messages.playerRegion} tabIndex={-1}>
      {empty ? (
        <View id="player-now" dataSet={{ empty: '1' }}>
          <Text id="player-empty">{messages.playerEmpty}</Text>
        </View>
      ) : (
        <>
          <View
            id="player-art"
            dataSet={{ playing: playback.playing ? '1' : '0' }}
            accessibilityRole="button"
            accessibilityLabel={messages.openFullPlayer}
            tabIndex={0}
            onClick={openFull}
            onKeyDown={(event) => {
              if (event.key === 'Enter' || event.key === ' ') {
                event.preventDefault();
                openFull();
              }
            }}
          >
            <CoverTile tone={playback.coverTone} label={playback.title} size="bar" artUrl={playback.coverUrl} />
          </View>
          <View
            id="player-meta"
            accessibilityRole="button"
            accessibilityLabel={messages.openFullPlayer}
            tabIndex={0}
            onClick={openFull}
            onKeyDown={(event) => {
              if (event.key === 'Enter' || event.key === ' ') {
                event.preventDefault();
                openFull();
              }
            }}
          >
            <Text id="player-title">{playback.title}</Text>
            <Text id="player-artist">{playback.artistName}</Text>
            {albumTitle !== undefined && albumTitle !== '' ? <Text id="player-album">{albumTitle}</Text> : null}
          </View>
        </>
      )}
      <View id="player-transport">
        <ControlButton id="player-prev" label={messages.previous} onPress={onPrevious} disabled={empty} />
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
      <View id="player-progress">
        {empty ? null : (
          <Text id="player-time-elapsed" dataSet={{ scrubberTime: '1' }}>
            {formatDuration(playback.positionMs)}
          </Text>
        )}
        <View
          id="player-scrubber"
          accessibilityRole="slider"
          accessibilityLabel={messages.progress}
          accessibilityValue={{
            min: 0,
            max: playback.durationMs,
            now: playback.positionMs,
            text: `${formatDuration(playback.positionMs)} of ${formatDuration(playback.durationMs)}`,
          }}
          dataSet={{ progress: `${Math.round(progress * 100)}` }}
          tabIndex={empty ? -1 : 0}
          onClick={seekFromEvent}
          onKeyDown={seekByKeyboard}
        >
          <View id="player-progress-track">
            <View
              id="player-progress-fill"
              dataSet={{ fill: `${Math.round(progress * 100)}` }}
              style={{ width: `${Math.round(progress * 100)}%` }}
            />
          </View>
        </View>
        {empty ? null : (
          <Text id="player-time-total" dataSet={{ scrubberTime: '1' }}>
            {formatDuration(playback.durationMs)}
          </Text>
        )}
      </View>
      <View id="player-actions">
        {empty ? null : (
          <View
            id="player-expand"
            dataSet={{ playerControl: 'plain', expand: '1' }}
            accessibilityRole="button"
            accessibilityLabel={messages.openFullPlayer}
            tabIndex={0}
            onClick={openFull}
            onKeyDown={(event) => {
              if (event.key === 'Enter' || event.key === ' ') {
                event.preventDefault();
                openFull();
              }
            }}
          >
            <Text dataSet={{ controlLabel: '1' }}>{messages.openFullPlayer}</Text>
          </View>
        )}
        {empty || compact ? null : (
          <View
            id="player-lyrics"
            dataSet={{ playerControl: 'plain', lyrics: '1' }}
            accessibilityRole="button"
            accessibilityLabel={messages.lyrics}
            tabIndex={0}
            onClick={openFull}
            onKeyDown={(event) => {
              if (event.key === 'Enter' || event.key === ' ') {
                event.preventDefault();
                openFull();
              }
            }}
          >
            <Text dataSet={{ controlLabel: '1' }}>{messages.lyrics}</Text>
          </View>
        )}
        {empty || compact || volume === undefined || onVolume === undefined ? null : (
          <View id="player-volume" dataSet={{ volume: '1' }}>
            <Text id="player-volume-icon" aria-hidden="true">
              ♪
            </Text>
            <input
              id="player-volume-range"
              type="range"
              min={0}
              max={1}
              step={0.01}
              value={volume}
              aria-label={messages.volume}
              onChange={(event) => {
                onVolume(Number(event.currentTarget.value));
              }}
            />
          </View>
        )}
        {empty || compact ? null : <View id="player-device" dataSet={{ deviceSlot: 'empty' }} />}
        <ControlButton id="player-queue" label={messages.queue} onPress={onToggleQueue} disabled={empty} />
      </View>
    </View>
  );
}

type ControlButtonProps = {
  id: string;
  label: string;
  onPress?: (() => void) | undefined;
  disabled?: boolean | undefined;
  primary?: boolean | undefined;
  playing?: boolean | undefined;
};

function ControlButton({ id, label, onPress, disabled = false, primary = false, playing = false }: ControlButtonProps) {
  const control = (
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
  /* The hex clip-path clips every paint of the button itself, so the focus
     ring lives on this square wrapper (design-language §8). */
  if (!primary) {
    return control;
  }
  return <View dataSet={{ hexWrap: '1' }}>{control}</View>;
}

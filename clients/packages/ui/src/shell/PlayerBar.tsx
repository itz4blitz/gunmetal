import { useLayoutEffect, useState } from 'react';
import { Text, View } from 'react-native-web';
import type { PlayerSnapshot } from '../../../ports/src/provisional/player.ts';
import type { ShellMessages } from '../messages/en/shell.ts';
import { CoverTile } from './destinations/CoverTile.tsx';
import { Icon, type IconName } from './Icon.tsx';
import { formatDuration } from './format.ts';

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

  /* The marquee engages only on a measured overflow. The visible width comes
     from the meta block's content box (the clipping parent), so re-measuring
     while the title is expanded stays correct; the shift is written as a CSS
     custom property so the motion itself stays CSS-only and pauses on hover
     or focus (see the gm-title-marquee rules in the demo stylesheet). */
  useLayoutEffect(() => {
    if (empty) {
      return;
    }
    const measure = () => {
      const el = globalThis.document.getElementById('player-title');
      const clip = el?.parentElement ?? null;
      if (el === null || clip === null || clip.clientWidth <= 0) {
        return;
      }
      const padding = globalThis.getComputedStyle(clip);
      const available = Math.max(
        0,
        Math.round(clip.clientWidth - parseFloat(padding.paddingLeft) - parseFloat(padding.paddingRight)),
      );
      const shift = titleMarqueeShift(el.scrollWidth, available);
      if (shift === null) {
        el.style.removeProperty('--gm-title-shift');
        return;
      }
      el.style.setProperty('--gm-title-shift', `${shift}px`);
    };
    measure();
    globalThis.addEventListener('resize', measure);
    return () => {
      globalThis.removeEventListener('resize', measure);
    };
  }, [empty, playback.title]);

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

  const secondary =
    albumTitle !== undefined && albumTitle !== '' ? `${playback.artistName} · ${albumTitle}` : playback.artistName;

  return (
    <View
      id="player-bar"
      accessibilityRole="region"
      accessibilityLabel={messages.playerRegion}
      tabIndex={-1}
      dataSet={{ barEmpty: empty ? '1' : '0' }}
    >
      {/* —— Left: art + what is playing (or the idle state) —— */}
      <View id="player-left">
        {empty ? (
          <>
            <View id="player-art-empty" />
            <View id="player-meta" dataSet={{ empty: '1' }}>
              <Text id="player-empty">{messages.playerEmpty}</Text>
            </View>
          </>
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
              {/* Shown on hover and focus: the artwork opens the full player. */}
              <View dataSet={{ artExpand: '1' }} aria-hidden={true}>
                <Icon name="expand" size={18} />
              </View>
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
              <Text id="player-artist">{secondary}</Text>
            </View>
          </>
        )}
      </View>

      {/* —— Centre: transport stacked over the scrubber, one optical centre —— */}
      <View id="player-center">
        <View id="player-transport">
          <ControlButton
            id="player-prev"
            label={messages.previous}
            onPress={onPrevious}
            disabled={empty}
            glyph="previous"
          />
          <ControlButton
            id="shell-play"
            label={playback.playing ? messages.pause : messages.play}
            onPress={onPlayPause}
            disabled={empty}
            primary
            playing={playback.playing}
            idle={empty}
          />
          <ControlButton id="player-next" label={messages.next} onPress={onNext} disabled={empty} glyph="next" />
        </View>
        <View id="player-progress" dataSet={{ barEmpty: empty ? '1' : '0' }}>
          <Text id="player-time-elapsed" dataSet={{ scrubberTime: '1' }}>
            {formatDuration(playback.positionMs)}
          </Text>
          <View
            id="player-scrubber"
            accessibilityRole="slider"
            accessibilityLabel={messages.progress}
            aria-valuemin={0}
            aria-valuemax={playback.durationMs}
            aria-valuenow={Math.round(playback.positionMs)}
            aria-valuetext={`${formatDuration(playback.positionMs)} of ${formatDuration(playback.durationMs)}`}
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
          <Text id="player-time-total" dataSet={{ scrubberTime: '1' }}>
            {formatDuration(playback.durationMs)}
          </Text>
        </View>
      </View>

      {/* —— Right: expand · lyrics · volume · queue (extras sleep when empty) —— */}
      <View id="player-actions">
        {empty || compact ? null : (
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
            <Icon name="expand" size={18} />
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
            <Icon name="lyrics" size={18} />
          </View>
        )}
        {empty || compact || volume === undefined || onVolume === undefined ? null : (
          <VolumeControl messages={messages} volume={volume} onVolume={onVolume} />
        )}
        <ControlButton
          id="player-queue"
          label={messages.queue}
          onPress={onToggleQueue}
          disabled={false}
          glyph="queue"
        />
      </View>
    </View>
  );
}

type VolumeControlProps = {
  messages: ShellMessages;
  volume: number;
  onVolume: (volume: number) => void;
};

/* The speaker button silences the output and brings it back to where it was:
   the level it returns to is the last audible one this control saw. */
function VolumeControl({ messages, volume, onVolume }: VolumeControlProps) {
  const [audibleVolume, setAudibleVolume] = useState(defaultAudibleVolume);
  const muted = volume === 0;
  const toggleMute = () => {
    if (volume > 0) {
      setAudibleVolume(volume);
    }
    onVolume(toggledVolume(volume, audibleVolume));
  };
  return (
    <View id="player-volume" dataSet={{ volume: '1' }}>
      <View
        id="player-volume-icon"
        dataSet={{ muted: muted ? '1' : '0' }}
        accessibilityRole="button"
        accessibilityLabel={muted ? messages.unmute : messages.mute}
        tabIndex={0}
        onClick={toggleMute}
        onKeyDown={(event) => {
          if (event.key === 'Enter' || event.key === ' ') {
            event.preventDefault();
            toggleMute();
          }
        }}
      >
        <Icon name={muted ? 'mute' : 'volume'} size={18} />
      </View>
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
  );
}

type ControlButtonProps = {
  id: string;
  label: string;
  onPress?: (() => void) | undefined;
  disabled?: boolean | undefined;
  primary?: boolean | undefined;
  playing?: boolean | undefined;
  idle?: boolean | undefined;
  glyph?: IconName | undefined;
};

function ControlButton({
  id,
  label,
  onPress,
  disabled = false,
  primary = false,
  playing = false,
  idle = false,
  glyph,
}: ControlButtonProps) {
  const control = (
    <View
      id={id}
      dataSet={{
        playerControl: primary ? 'primary' : 'plain',
        disabled: disabled ? '1' : '0',
        playing: playing ? '1' : '0',
        idle: idle ? '1' : '0',
      }}
      accessibilityRole="button"
      accessibilityLabel={label}
      aria-disabled={disabled ? true : undefined}
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
      {glyph === undefined ? <Text dataSet={{ controlLabel: '1' }}>{label}</Text> : <Icon name={glyph} size={20} />}
    </View>
  );
  /* The hex clip-path clips every paint of the button itself, so the focus
     ring lives on this square wrapper (design-language §8). */
  if (!primary) {
    return control;
  }
  return <View dataSet={{ hexWrap: '1' }}>{control}</View>;
}

/** The level an unmute returns to when this bar never saw an audible one. */
export const defaultAudibleVolume = 0.8;

/** Where the speaker button sends the volume: to silence when it is audible,
 * and back to the remembered level when it is already silent. */
export function toggledVolume(volume: number, remembered: number): number {
  return volume > 0 ? 0 : remembered;
}

/** Returns the pixel shift for a marquee, or null when the title fits. */
export function titleMarqueeShift(scrollWidth: number, available: number): number | null {
  const slack = 2;
  if (scrollWidth <= available + slack) {
    return null;
  }
  return scrollWidth - available + slack;
}

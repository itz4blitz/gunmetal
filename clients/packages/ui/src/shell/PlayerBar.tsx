import { useLayoutEffect, useState } from 'react';
import { Text, View } from 'react-native-web';
import type { PlayerSnapshot } from '../../../ports/src/provisional/player.ts';
import type { ShellMessages } from '../messages/en/shell.ts';
import { CoverTile } from './destinations/CoverTile.tsx';
import { Icon, type IconName } from './Icon.tsx';
import { RepeatGlyph } from './player-glyphs.tsx';
import { formatDuration } from './format.ts';
import { usePositionMs, type PositionClock } from './position-clock.ts';

export type PlayerBarProps = {
  messages: ShellMessages;
  playback: PlayerSnapshot;
  albumTitle?: string | undefined;
  compact?: boolean | undefined;
  volume?: number | undefined;
  onVolume?: ((volume: number) => void) | undefined;
  /** Whether the output is silenced; wired with onMuted this state is honest. */
  muted?: boolean | undefined;
  onMuted?: ((muted: boolean) => void) | undefined;
  onSeek?: ((positionMs: number) => void) | undefined;
  onPlayPause?: (() => void) | undefined;
  onPrevious?: (() => void) | undefined;
  onNext?: (() => void) | undefined;
  onToggleShuffle?: (() => void) | undefined;
  onCycleRepeat?: (() => void) | undefined;
  onToggleQueue?: (() => void) | undefined;
  onOpenFull?: (() => void) | undefined;
  /** The composition root's position clock; without it the snapshot's position is shown. */
  clock?: PositionClock | undefined;
};

export function PlayerBar({
  messages,
  playback,
  albumTitle,
  compact = false,
  volume,
  onVolume,
  muted,
  onMuted,
  onSeek,
  onPlayPause,
  onPrevious,
  onNext,
  onToggleShuffle,
  onCycleRepeat,
  onToggleQueue,
  onOpenFull,
  clock,
}: PlayerBarProps) {
  const empty = playback.trackId === undefined;
  /* An empty album title is not a credit. The name stays on its own element
     so the opener's label cannot swallow it, while the credit line's text
     stays "artist · album". */
  const shownAlbum = albumTitle !== undefined && albumTitle !== '' ? albumTitle : undefined;

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
      const clip = globalThis.document.getElementById('player-title')?.parentElement ?? null;
      if (clip === null || clip.clientWidth <= 0) {
        return;
      }
      const padding = globalThis.getComputedStyle(clip);
      const available = Math.max(
        0,
        Math.round(clip.clientWidth - parseFloat(padding.paddingLeft) - parseFloat(padding.paddingRight)),
      );
      /* The clip is the credit block whose first child is the title. */
      const el = clip.firstElementChild as HTMLElement;
      const shift = titleMarqueeShift(el.scrollWidth, available);
      if (shift === null) {
        el?.style.removeProperty('--gm-title-shift');
        return;
      }
      el?.style.setProperty('--gm-title-shift', `${shift}px`);
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

  /* One honest state line: the engine's failure reason, or the buffering it
     reports. Announced without moving focus (design-language §11). */
  const stateLine =
    playback.playbackError !== undefined
      ? {
          kind: 'error' as const,
          text: playback.playbackError === '' ? messages.playerPlaybackFailed : playback.playbackError,
        }
      : playback.buffering === true
        ? { kind: 'buffering' as const, text: messages.playerBuffering }
        : null;

  return (
    <View
      id="player-bar"
      accessibilityRole="region"
      accessibilityLabel={messages.playerRegion}
      tabIndex={-1}
      dataSet={{
        barEmpty: empty ? '1' : '0',
        buffering: playback.buffering === true ? '1' : '0',
        errored: playback.playbackError !== undefined ? '1' : '0',
      }}
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
              <Text id="player-title" accessibilityRole="header">
                {playback.title}
              </Text>
              <Text
                id="player-artist"
                accessibilityRole={shownAlbum === undefined ? 'group' : undefined}
                accessibilityLabel={shownAlbum === undefined ? playback.artistName : undefined}
              >
                {shownAlbum === undefined ? (
                  playback.artistName
                ) : (
                  <>
                    <Text accessibilityRole="group" accessibilityLabel={playback.artistName}>
                      {playback.artistName}
                    </Text>
                    {' · '}
                    <Text accessibilityRole="group" accessibilityLabel={shownAlbum}>
                      {shownAlbum}
                    </Text>
                  </>
                )}
              </Text>
              {stateLine === null ? null : (
                <Text id="player-state" accessibilityRole="status" dataSet={{ playerState: stateLine.kind }}>
                  {stateLine.text}
                </Text>
              )}
            </View>
          </>
        )}
      </View>

      {/* —— Centre: transport stacked over the scrubber, one optical centre —— */}
      <View id="player-center">
        <View id="player-transport">
          <ControlButton
            id="player-shuffle"
            label={messages.playerShuffle}
            onPress={onToggleShuffle}
            disabled={empty}
            pressed={playback.shuffleOn === true}
            glyph="shuffle"
          />
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
          <ControlButton
            id="player-repeat"
            label={
              playback.repeatMode === 'one'
                ? messages.playerRepeatOne
                : playback.repeatMode === 'all'
                  ? messages.playerRepeatAll
                  : messages.playerRepeat
            }
            onPress={onCycleRepeat}
            disabled={empty}
            pressed={playback.repeatMode === 'all' || playback.repeatMode === 'one'}
            repeatOne={playback.repeatMode === 'one'}
            glyph="repeat"
          />
        </View>
        <ProgressZone messages={messages} playback={playback} empty={empty} onSeek={onSeek} clock={clock} />
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
          <VolumeControl messages={messages} volume={volume} muted={muted} onMuted={onMuted} onVolume={onVolume} />
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

type ProgressZoneProps = {
  messages: ShellMessages;
  playback: PlayerSnapshot;
  empty: boolean;
  onSeek?: ((positionMs: number) => void) | undefined;
  clock: PositionClock | undefined;
};

/* The elapsed time, the scrubber and the total. With a position clock this
   zone — and only this zone — follows the frames; without one it reads the
   snapshot's position, exactly as before. */
function ProgressZone({ messages, playback, empty, onSeek, clock }: ProgressZoneProps) {
  const positionMs = usePositionMs(clock, playback.positionMs);
  const durationMs = playback.durationMs;
  const progress = durationMs > 0 ? Math.min(1, positionMs / durationMs) : 0;
  const seekFromEvent = (event: { currentTarget: HTMLElement; clientX: number }) => {
    if (onSeek === undefined || empty || durationMs <= 0) {
      return;
    }
    const rect = event.currentTarget.getBoundingClientRect();
    if (rect.width <= 0) {
      return;
    }
    const fraction = Math.max(0, Math.min(1, (event.clientX - rect.left) / rect.width));
    onSeek(Math.round(fraction * durationMs));
  };
  const seekByKeyboard = (event: { key: string; preventDefault: () => void }) => {
    if (onSeek === undefined || empty || durationMs <= 0) {
      return;
    }
    const step = 5000;
    if (event.key === 'ArrowLeft') {
      event.preventDefault();
      onSeek(Math.max(0, positionMs - step));
      return;
    }
    if (event.key === 'ArrowRight') {
      event.preventDefault();
      onSeek(Math.min(durationMs, positionMs + step));
    }
  };
  return (
    <View id="player-progress" dataSet={{ barEmpty: empty ? '1' : '0' }}>
      <Text id="player-time-elapsed" dataSet={{ scrubberTime: '1' }}>
        {formatDuration(positionMs)}
      </Text>
      <View
        id="player-scrubber"
        accessibilityRole="slider"
        accessibilityLabel={messages.progress}
        aria-valuemin={0}
        aria-valuemax={durationMs}
        aria-valuenow={Math.round(positionMs)}
        aria-valuetext={`${formatDuration(positionMs)} of ${formatDuration(durationMs)}`}
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
        {formatDuration(durationMs)}
      </Text>
    </View>
  );
}

type VolumeControlProps = {
  messages: ShellMessages;
  volume: number;
  muted?: boolean | undefined;
  onMuted?: ((muted: boolean) => void) | undefined;
  onVolume: (volume: number) => void;
};

/* The speaker button silences the output and brings it back to where it was:
   the level it returns to is the last audible one this control saw. When the
   composition root owns the mute, the button says the truth it is given. */
function VolumeControl({ messages, volume, muted, onMuted, onVolume }: VolumeControlProps) {
  const [audibleVolume, setAudibleVolume] = useState(defaultAudibleVolume);
  const owned = onMuted !== undefined;
  const silenced = owned ? muted === true : volume === 0;
  const toggleMute = () => {
    if (owned) {
      onMuted(!(muted === true));
      return;
    }
    if (volume > 0) {
      setAudibleVolume(volume);
    }
    onVolume(toggledVolume(volume, audibleVolume));
  };
  return (
    <View id="player-volume" dataSet={{ volume: '1' }}>
      <View
        id="player-volume-icon"
        dataSet={{ muted: silenced ? '1' : '0' }}
        accessibilityRole="button"
        accessibilityLabel={silenced ? messages.unmute : messages.mute}
        aria-pressed={owned ? silenced : undefined}
        tabIndex={0}
        onClick={toggleMute}
        onKeyDown={(event) => {
          if (event.key === 'Enter' || event.key === ' ') {
            event.preventDefault();
            toggleMute();
          }
        }}
      >
        <Icon name={silenced ? 'mute' : 'volume'} size={18} />
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
          const level = Number(event.currentTarget.value);
          if (owned && silenced && level > 0) {
            onMuted(false);
          }
          onVolume(level);
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
  /** A toggle control says whether it is on. */
  pressed?: boolean | undefined;
  /** The repeat control draws its own glyph: plain arcs, or with the one. */
  repeatOne?: boolean | undefined;
  glyph?: IconName | 'repeat' | undefined;
};

function ControlButton({
  id,
  label,
  onPress,
  disabled = false,
  primary = false,
  playing = false,
  idle = false,
  pressed,
  repeatOne,
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
        ...(pressed === undefined ? {} : { pressed: pressed ? '1' : '0' }),
      }}
      accessibilityRole="button"
      accessibilityLabel={label}
      aria-disabled={disabled ? true : undefined}
      aria-pressed={pressed === undefined ? undefined : pressed}
      tabIndex={disabled ? -1 : 0}
      onClick={disabled ? undefined : onPress}
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
      {glyph === undefined ? (
        <Text dataSet={{ controlLabel: '1' }}>{label}</Text>
      ) : glyph === 'repeat' ? (
        <RepeatGlyph one={repeatOne === true} size={20} />
      ) : (
        <Icon name={glyph} size={20} />
      )}
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

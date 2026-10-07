import { useLayoutEffect, useState } from 'react';
import { Text, View } from 'react-native-web';
import type { PlayerSnapshot } from '../../../ports/src/provisional/player.ts';
import type { ShellMessages } from '../messages/en/shell.ts';
import { CoverTile } from './destinations/CoverTile.tsx';
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
  /** Starts the featured album from the empty state (steel play button). */
  onPlayFirst?: (() => void) | undefined;
  onPrevious?: (() => void) | undefined;
  onNext?: (() => void) | undefined;
  onToggleQueue?: (() => void) | undefined;
  onOpenFull?: (() => void) | undefined;
};

/* Inline SVG glyphs (design-language §10: elements, no style inside). */
function IconGlyph({ path, label }: { path: string; label: string }) {
  return (
    <svg
      viewBox="0 0 24 24"
      width="18"
      height="18"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-label={label}
      role="img"
    >
      <path d={path} />
    </svg>
  );
}

const GLYPH_EXPAND = 'M6 14l6-6 6 6';
const GLYPH_LYRICS = 'M4 6h16M4 11h16M4 16h10';
const GLYPH_VOLUME = 'M4 9v6h4l5 4V5L8 9H4z M16.5 8.5a5 5 0 0 1 0 7';
const GLYPH_QUEUE = 'M4 6h16M4 11h16M4 16h9M18 14v6M15 17h6';

export function PlayerBar({
  messages,
  playback,
  albumTitle,
  compact = false,
  volume,
  onVolume,
  onSeek,
  onPlayPause,
  onPlayFirst,
  onPrevious,
  onNext,
  onToggleQueue,
  onOpenFull,
}: PlayerBarProps) {
  const empty = playback.trackId === undefined;
  const progress = playback.durationMs > 0 ? Math.min(1, playback.positionMs / playback.durationMs) : 0;
  const [marqueeShift, setMarqueeShift] = useState(0);

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
        setMarqueeShift(0);
        return;
      }
      el.style.setProperty('--gm-title-shift', `${shift}px`);
      setMarqueeShift(shift);
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
          <ControlButton id="player-prev" label={messages.previous} onPress={onPrevious} disabled={empty} />
          <ControlButton
            id="shell-play"
            label={playback.playing ? messages.pause : messages.play}
            onPress={() => {
              if (empty) {
                onPlayFirst?.();
                return;
              }
              onPlayPause?.();
            }}
            disabled={empty && onPlayFirst === undefined}
            primary
            playing={playback.playing}
            idle={empty}
          />
          <ControlButton id="player-next" label={messages.next} onPress={onNext} disabled={empty} />
        </View>
        <View id="player-progress" dataSet={{ barEmpty: empty ? '1' : '0' }}>
          <Text id="player-time-elapsed" dataSet={{ scrubberTime: '1' }}>
            {formatDuration(playback.positionMs)}
          </Text>
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
            <IconGlyph path={GLYPH_EXPAND} label={messages.openFullPlayer} />
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
            <IconGlyph path={GLYPH_LYRICS} label={messages.lyrics} />
          </View>
        )}
        {empty || compact || volume === undefined || onVolume === undefined ? null : (
          <View id="player-volume" dataSet={{ volume: '1' }}>
            <View id="player-volume-icon">
              <IconGlyph path={GLYPH_VOLUME} label={messages.volume} />
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
        )}
        <ControlButton
          id="player-queue"
          label={messages.queue}
          onPress={onToggleQueue}
          disabled={false}
          glyph={GLYPH_QUEUE}
        />
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
  idle?: boolean | undefined;
  glyph?: string | undefined;
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
      {glyph === undefined ? (
        <Text dataSet={{ controlLabel: '1' }}>{label}</Text>
      ) : (
        <IconGlyph path={glyph} label={label} />
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

/** Returns the pixel shift for a marquee, or null when the title fits. */
export function titleMarqueeShift(scrollWidth: number, available: number): number | null {
  const slack = 2;
  if (scrollWidth <= available + slack) {
    return null;
  }
  return scrollWidth - available + slack;
}

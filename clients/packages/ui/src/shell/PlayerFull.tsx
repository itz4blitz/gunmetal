import { useEffect, useRef, useState } from 'react';
import { Text, View } from 'react-native-web';
import type { ShellMessages } from '../messages/en/shell.ts';
import { CoverTile } from './destinations/CoverTile.tsx';
import { formatDuration } from './format.ts';
import { noLyrics, type LyricsResolver } from './content.ts';
import { Icon, type IconName } from './Icon.tsx';
import { RepeatGlyph } from './player-glyphs.tsx';
import { LyricsPane } from './LyricsPane.tsx';
import { progressFillWidth, usePositionMs, type PositionClock } from './position-clock.ts';
import type { SyncedLine, TimedLyricsResolver } from './synced-lyrics.ts';
import type { PlayerQueueLine, PlayerSnapshot } from '../../../ports/src/provisional/player.ts';

export type PlayerPlacement = 'overlay' | 'pane';

export type PlayerFullProps = {
  lyricsFor?: LyricsResolver | undefined;
  /** The resolver's timestamped sheet for synced tracks; the plain one is the fallback. */
  timedLyricsFor?: TimedLyricsResolver | undefined;
  messages: ShellMessages;
  playback: PlayerSnapshot;
  open: boolean;
  placement?: PlayerPlacement | undefined;
  albumTitle?: string | undefined;
  volume?: number | undefined;
  onVolume?: ((volume: number) => void) | undefined;
  /** Whether the output is silenced; wired with onMuted this state is honest. */
  muted?: boolean | undefined;
  onMuted?: ((muted: boolean) => void) | undefined;
  /** Seek within the playing track; without it the scrubber only shows progress. */
  onSeek?: ((positionMs: number) => void) | undefined;
  onClose: () => void;
  onPlayPause?: (() => void) | undefined;
  onPrevious?: (() => void) | undefined;
  onNext?: (() => void) | undefined;
  onToggleShuffle?: (() => void) | undefined;
  onCycleRepeat?: (() => void) | undefined;
  onToggleQueue?: (() => void) | undefined;
  /** The composition root's position clock; without it the given position is shown. */
  clock?: PositionClock | undefined;
  /** The playing position when no clock is wired (milliseconds). */
  positionMs?: number | undefined;
};

function nextPlayerQueueLine(playback: PlayerSnapshot): PlayerQueueLine | undefined {
  const index = playback.queue.findIndex((line) => line.trackId === playback.trackId);
  if (index < 0) {
    return undefined;
  }
  return playback.queue[index + 1];
}

/** How long the view takes to fold away: the sheet motion token (--gm-motion-sheet). */
const LEAVE_MS = 200;

/** True where the leave motion can run: a browser that has not asked for reduced motion. */
function motionAllowed(): boolean {
  return (
    typeof globalThis.matchMedia === 'function' && !globalThis.matchMedia('(prefers-reduced-motion: reduce)').matches
  );
}

/** How far each arrow key moves the scrubber. */
const SEEK_STEPS: Readonly<Record<string, number>> = { ArrowLeft: -5000, ArrowRight: 5000 };

/** Enter and Space activate a control; every other key passes through. */
function onActivate(action: () => void) {
  return (event: { key: string; preventDefault: () => void }) => {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      action();
    }
  };
}

/**
 * Where Tab goes inside a dialog's focusable nodes: the last one when
 * leaving the first backwards, the first one when leaving the last (or
 * when the focus is outside the dialog at all), and nowhere in between —
 * the browser keeps the natural order.
 */
export function tabWrapTarget(
  nodes: readonly HTMLElement[],
  active: Element | null,
  shiftKey: boolean,
): HTMLElement | undefined {
  if (nodes.length === 0) {
    return undefined;
  }
  /* The bounds are of the list itself, which is not empty here. */
  const first = nodes[0] as HTMLElement;
  const last = nodes[nodes.length - 1] as HTMLElement;
  const inside = active instanceof HTMLElement && nodes.includes(active);
  if (shiftKey) {
    return active === first || !inside ? last : undefined;
  }
  return active === last || !inside ? first : undefined;
}

/**
 * The now-playing view (SUR-010). Three regions inside one body: the chrome
 * (heading and collapse), the stage (the artwork, or the lyrics that take its
 * place) and the console (title block, scrubber, transport, secondary row,
 * what plays next). The stylesheet sets the console under the stage on a
 * narrow player and beside it on a wide one. As a dialog it is modal: the
 * focus enters at the collapse control, Tab stays inside, and closing hands
 * the focus back to the control that opened it.
 */
export function PlayerFull({
  lyricsFor = () => noLyrics,
  timedLyricsFor,
  messages,
  playback,
  open,
  placement = 'overlay',
  albumTitle,
  volume,
  onVolume,
  muted,
  onMuted,
  onSeek,
  onClose,
  onPlayPause,
  onPrevious,
  onNext,
  onToggleShuffle,
  onCycleRepeat,
  onToggleQueue,
  clock,
  positionMs,
}: PlayerFullProps) {
  /* Closing keeps the view mounted for the length of the sheet motion, marked
     as leaving, so it can fold back toward the bar it grew from. */
  const [wasOpen, setWasOpen] = useState(open);
  const [leaving, setLeaving] = useState(false);
  if (open !== wasOpen) {
    setWasOpen(open);
    setLeaving(!open && motionAllowed());
  }
  useEffect(() => {
    if (!leaving) {
      return;
    }
    const timer = setTimeout(() => {
      setLeaving(false);
    }, LEAVE_MS);
    return () => {
      clearTimeout(timer);
    };
  }, [leaving]);
  const [lyricsOpen, setLyricsOpen] = useState(false);
  const canLyrics = playback.lyricsKind === 'plain' || playback.lyricsKind === 'synced';
  useEffect(() => {
    if (!canLyrics) {
      setLyricsOpen(false);
    }
  }, [canLyrics]);
  /* Playback can open the queue on its own. Until the queue is asked for
     from this view its sheet stays underneath; the request is forgotten
     when the queue closes or the view is put away. */
  const [queueRaised, setQueueRaised] = useState(false);
  const queueOpen = playback.queueOpen;
  useEffect(() => {
    if (!open || !queueOpen) {
      setQueueRaised(false);
    }
  }, [open, queueOpen]);
  /* The raised sheet is a sibling of this dialog, so the dialog's own focus
     work pauses while it is up: the sheet's close control takes the focus
     when it rises and the queue control gets it back when it goes down. */
  const sheetOver = open && queueOpen && queueRaised;
  const sheetWasOver = useRef(false);
  useEffect(() => {
    if (!open) {
      sheetWasOver.current = false;
      return;
    }
    if (sheetOver) {
      sheetWasOver.current = true;
      globalThis.document.getElementById('queue-close')?.focus();
      return;
    }
    if (!sheetWasOver.current) {
      return;
    }
    sheetWasOver.current = false;
    globalThis.document.getElementById('player-full-queue')?.focus();
  }, [open, sheetOver]);
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
  /* The dialog owns the focus while it is open: it takes it at the collapse
     control and hands it back where it came from (design-language §8). */
  useEffect(() => {
    if (!open) {
      return;
    }
    const previous = globalThis.document.activeElement as HTMLElement | null;
    const first = globalThis.document
      .getElementById('player-full')
      ?.querySelector<HTMLElement>('[role="button"][tabindex="0"]');
    first?.focus();
    return () => {
      if (previous !== null && globalThis.document.contains(previous)) {
        previous.focus();
      }
    };
  }, [open]);
  /* Tab never leaves the dialog while it is open: at either end it wraps to
     the other, and a focus that escaped (a click on the scrim) comes back.
     While the queue sheet is raised the trap stands aside instead: that
     sheet is a sibling surface and its lines are the natural next stops. */
  const trapTab = (event: {
    key: string;
    shiftKey?: boolean;
    preventDefault: () => void;
    currentTarget: HTMLElement;
  }) => {
    if (event.key !== 'Tab' || sheetOver) {
      return;
    }
    const target = tabWrapTarget(
      [...event.currentTarget.querySelectorAll<HTMLElement>('[tabindex="0"], input')].filter(
        (node) => node.getAttribute('aria-disabled') !== 'true',
      ),
      globalThis.document.activeElement,
      event.shiftKey === true,
    );
    if (target !== undefined) {
      event.preventDefault();
      target.focus();
    }
  };

  if (playback.trackId === undefined || !(open || leaving)) {
    return null;
  }

  const upNext = nextPlayerQueueLine(playback);
  const fromLabel = albumTitle !== undefined && albumTitle !== '' ? `${messages.playingFrom} ${albumTitle}` : undefined;
  const lyricsShown = canLyrics && lyricsOpen;
  /* A synced resolver's timestamped sheet wins over the plain verses; a
     track it has nothing for falls back to the plain resolver's answer. */
  const timed =
    playback.lyricsKind === 'synced' && timedLyricsFor !== undefined
      ? timedLyricsFor(playback.trackId, 'synced')
      : undefined;
  const lyrics =
    timed !== undefined && timed.length > 0
      ? timed.map((line) => line.text)
      : canLyrics
        ? lyricsFor(playback.trackId, playback.lyricsKind)
        : noLyrics;
  const stateLine =
    playback.playbackError !== undefined
      ? {
          kind: 'error' as const,
          text: playback.playbackError === '' ? messages.playerPlaybackFailed : playback.playbackError,
        }
      : playback.buffering === true
        ? { kind: 'buffering' as const, text: messages.playerBuffering }
        : null;
  const seekable = onSeek !== undefined && playback.durationMs > 0;
  const toggleLyrics = () => {
    if (canLyrics) {
      setLyricsOpen((shown) => !shown);
    }
  };
  const toggleQueue = () => {
    setQueueRaised(true);
    /* A sheet left open underneath is brought up, not toggled shut. */
    if (queueOpen && !queueRaised) {
      return;
    }
    onToggleQueue?.();
  };

  return (
    <>
      {placement === 'overlay' ? (
        <View id="player-full-scrim" dataSet={{ open: open ? '1' : '0' }} onClick={onClose} />
      ) : null}
      <View
        id="player-full"
        accessibilityRole="dialog"
        accessibilityLabel={messages.playerFullRegion}
        aria-modal={true}
        dataSet={{
          open: open ? '1' : '0',
          placement,
          queueSheet: queueOpen && queueRaised ? 'over' : 'under',
          buffering: playback.buffering === true ? '1' : '0',
          errored: playback.playbackError !== undefined ? '1' : '0',
        }}
        onKeyDown={trapTab}
      >
        {/* Ambient artwork backdrop: the blurred cover sits under a heavy
            canvas veil, and the veil — not the artwork — carries the text
            contrast (design-language §5, scrim rule). */}
        <View id="player-full-ambient">
          {playback.coverUrl === '' ? null : (
            <img alt="" aria-hidden="true" data-ambient-art="1" src={playback.coverUrl} />
          )}
          <View dataSet={{ ambientVeil: '1' }} />
        </View>
        <View id="player-full-body">
          <View id="player-full-chrome">
            <Text id="player-full-heading" accessibilityRole="header">
              {messages.playerRegion}
            </Text>
            <View
              id="player-full-collapse"
              accessibilityRole="button"
              accessibilityLabel={messages.playerClose}
              tabIndex={0}
              onClick={onClose}
              onKeyDown={onActivate(onClose)}
            >
              <Icon name="collapse" size={22} />
            </View>
          </View>
          {/* Open lyrics take the artwork's place (player.md, "The full-screen
              player"); the cover stays mounted so the tint follows it. */}
          <View id="player-full-stage" dataSet={{ stage: lyricsShown ? 'lyrics' : 'art' }}>
            <View id="player-full-art">
              <CoverTile
                tone={playback.coverTone}
                label={playback.title}
                size="full"
                coverId={`cover-full-${playback.trackId}`}
                artUrl={playback.coverUrl}
              />
            </View>
            {canLyrics ? (
              <LiveLyrics
                clock={clock}
                positionMs={positionMs}
                playback={playback}
                timed={timed}
                id="player-full-lyrics"
                label={messages.lyrics}
                lines={lyrics}
                synced={playback.lyricsKind === 'synced'}
                open={lyricsOpen}
                empty={lyrics === noLyrics}
              />
            ) : null}
          </View>
          <View id="player-full-console">
            <View id="player-full-info">
              {fromLabel === undefined ? null : <Text id="player-full-from">{fromLabel}</Text>}
              <Text id="player-full-title">{playback.title}</Text>
              <Text id="player-full-artist">{playback.artistName}</Text>
              {stateLine === null ? null : (
                <Text id="player-full-state" accessibilityRole="status" dataSet={{ playerState: stateLine.kind }}>
                  {stateLine.text}
                </Text>
              )}
            </View>
            <LiveProgress
              clock={clock}
              positionMs={positionMs}
              playback={playback}
              messages={messages}
              seekable={seekable}
              onSeek={onSeek}
            />
            <View id="player-full-transport">
              <FullControl
                id="player-full-shuffle"
                label={messages.playerShuffle}
                onPress={onToggleShuffle}
                glyph="shuffle"
                pressed={playback.shuffleOn === true}
              />
              <FullControl id="player-full-skip-back" label={messages.previous} onPress={onPrevious} glyph="previous" />
              <FullControl
                id="player-full-play"
                label={playback.playing ? messages.pause : messages.play}
                onPress={onPlayPause}
                playing={playback.playing}
              />
              <FullControl id="player-full-skip-next" label={messages.next} onPress={onNext} glyph="next" />
              <FullControl
                id="player-full-repeat"
                label={
                  playback.repeatMode === 'one'
                    ? messages.playerRepeatOne
                    : playback.repeatMode === 'all'
                      ? messages.playerRepeatAll
                      : messages.playerRepeat
                }
                onPress={onCycleRepeat}
                glyph="repeat"
                pressed={playback.repeatMode === 'all' || playback.repeatMode === 'one'}
                repeatOne={playback.repeatMode === 'one'}
              />
            </View>
            <View id="player-full-footer">
              {volume === undefined || onVolume === undefined ? null : (
                <View id="player-full-volume" dataSet={{ volume: '1' }}>
                  {onMuted === undefined ? (
                    <View id="player-full-volume-icon" tabIndex={-1}>
                      <Icon name={(muted ?? false) ? 'mute' : 'volume'} size={20} />
                    </View>
                  ) : (
                    <View
                      id="player-full-volume-icon"
                      accessibilityRole="button"
                      accessibilityLabel={(muted ?? false) ? messages.unmute : messages.mute}
                      aria-pressed={(muted ?? false) ? true : false}
                      tabIndex={0}
                      onClick={() => {
                        onMuted(!(muted ?? false));
                      }}
                      onKeyDown={onActivate(() => {
                        onMuted(!(muted ?? false));
                      })}
                    >
                      <Icon name={(muted ?? false) ? 'mute' : 'volume'} size={20} />
                    </View>
                  )}
                  <input
                    id="player-full-volume-range"
                    type="range"
                    min={0}
                    max={1}
                    step={0.01}
                    value={volume}
                    aria-label={messages.volume}
                    onChange={(event) => {
                      const level = Number(event.currentTarget.value);
                      if ((muted ?? false) && level > 0) {
                        onMuted?.(false);
                      }
                      onVolume(level);
                    }}
                  />
                </View>
              )}
              <View id="player-full-footer-actions">
                <View
                  id="player-full-lyrics-toggle"
                  accessibilityRole="button"
                  accessibilityLabel={messages.lyrics}
                  aria-pressed={lyricsShown}
                  aria-disabled={canLyrics ? undefined : true}
                  tabIndex={0}
                  dataSet={{
                    lyricsToggle: lyricsShown ? '1' : '0',
                    lyricsAvailable: canLyrics ? '1' : '0',
                  }}
                  onClick={toggleLyrics}
                  onKeyDown={onActivate(toggleLyrics)}
                >
                  <Icon name="lyrics" size={20} />
                </View>
                <View
                  id="player-full-queue"
                  accessibilityRole="button"
                  accessibilityLabel={messages.queue}
                  aria-pressed={queueOpen}
                  tabIndex={0}
                  onClick={toggleQueue}
                  onKeyDown={onActivate(toggleQueue)}
                >
                  <Icon name="queue" size={20} />
                </View>
              </View>
            </View>
            {canLyrics ? null : <Text id="player-full-lyrics-unavailable">{messages.lyricsUnavailable}</Text>}
            {upNext === undefined ? null : (
              <View id="player-full-up-next">
                <Text id="player-full-up-next-label">{messages.queueHeading}</Text>
                <View id="player-full-up-next-line">
                  <CoverTile
                    tone={upNext.coverTone}
                    label={upNext.title}
                    size="row"
                    coverId={`cover-next-${upNext.trackId}`}
                    artUrl={upNext.coverUrl}
                  />
                  <Text id="player-full-up-next-title">{upNext.title}</Text>
                  <Text id="player-full-up-next-artist">{upNext.artistName}</Text>
                  <Text id="player-full-up-next-duration">{formatDuration(upNext.durationMs)}</Text>
                </View>
              </View>
            )}
          </View>
        </View>
      </View>
    </>
  );
}

type LiveProps = {
  clock: PositionClock | undefined;
  positionMs: number | undefined;
  playback: PlayerSnapshot;
};

/* The progress zone follows the frames when a clock is wired; without one it
   reads the position it was given, exactly as before. Only this zone and the
   lyric sheet re-render on a frame — not the dialog around them. */
function LiveProgress({
  clock,
  positionMs,
  playback,
  messages,
  seekable,
  onSeek,
}: LiveProps & {
  messages: ShellMessages;
  seekable: boolean;
  onSeek: ((positionMs: number) => void) | undefined;
}) {
  const at = usePositionMs(clock, positionMs ?? playback.positionMs);
  const durationMs = playback.durationMs;
  const progress = durationMs > 0 ? Math.min(1, at / durationMs) : 0;
  const remainingMs = durationMs > 0 ? Math.max(0, durationMs - at) : 0;
  const seekFromPointer = (event: { currentTarget: HTMLElement; clientX: number }) => {
    const rect = event.currentTarget.getBoundingClientRect();
    if (onSeek === undefined || durationMs <= 0 || rect.width <= 0) {
      return;
    }
    const fraction = Math.max(0, Math.min(1, (event.clientX - rect.left) / rect.width));
    onSeek(Math.round(fraction * durationMs));
  };
  const seekFromKey = (event: { key: string; preventDefault: () => void }) => {
    const step = SEEK_STEPS[event.key];
    if (onSeek === undefined || durationMs <= 0 || step === undefined) {
      return;
    }
    event.preventDefault();
    onSeek(Math.max(0, Math.min(durationMs, at + step)));
  };
  return (
    <View id="player-full-progress" dataSet={{ progress: `${Math.round(progress * 100)}` }}>
      <View
        id="player-full-scrubber"
        accessibilityRole="slider"
        accessibilityLabel={messages.progress}
        aria-valuemin={0}
        aria-valuemax={durationMs}
        aria-valuenow={Math.round(at)}
        aria-valuetext={`${formatDuration(at)} of ${formatDuration(durationMs)}`}
        dataSet={{ seekable: seekable ? '1' : '0' }}
        tabIndex={seekable ? 0 : -1}
        onClick={seekFromPointer}
        onKeyDown={seekFromKey}
      >
        <View id="player-full-progress-track">
          <View
            id="player-full-progress-fill"
            dataSet={{ fill: `${Math.round(progress * 100)}` }}
            style={{ width: progressFillWidth(at, durationMs) }}
          />
        </View>
      </View>
      <View id="player-full-time">
        <Text id="player-full-elapsed">{formatDuration(at)}</Text>
        <Text id="player-full-remaining">{formatDuration(remainingMs)}</Text>
      </View>
    </View>
  );
}

/* The lyric sheet follows the same clock: the pane owns the line highlight
   and the follow behaviour; this leaf only keeps the position honest. */
function LiveLyrics({
  clock,
  positionMs,
  playback,
  timed,
  ...pane
}: LiveProps & {
  timed: readonly SyncedLine[] | undefined;
  id: string;
  label: string;
  lines: readonly string[];
  synced: boolean;
  open: boolean;
  empty: boolean;
}) {
  const at = usePositionMs(clock, positionMs ?? playback.positionMs);
  return <LyricsPane {...pane} timedLines={timed} positionMs={at} />;
}

type FullControlProps = {
  id: string;
  label: string;
  onPress?: (() => void) | undefined;
  /** A plain control draws this icon; without one the control is the nut. */
  glyph?: IconName | 'repeat' | undefined;
  playing?: boolean | undefined;
  pressed?: boolean | undefined;
  repeatOne?: boolean | undefined;
};

function FullControl({ id, label, onPress, glyph, playing = false, pressed, repeatOne }: FullControlProps) {
  const press = () => {
    onPress?.();
  };
  const control = (
    <View
      id={id}
      dataSet={{
        playerControl: glyph === undefined ? 'primary' : 'plain',
        playing: playing ? '1' : '0',
        ...(pressed === undefined ? {} : { pressed: pressed ? '1' : '0' }),
      }}
      accessibilityRole="button"
      accessibilityLabel={label}
      aria-pressed={pressed === undefined ? undefined : pressed}
      tabIndex={0}
      onClick={press}
      onKeyDown={onActivate(press)}
    >
      {glyph === undefined ? (
        <Text dataSet={{ controlLabel: '1' }}>{label}</Text>
      ) : glyph === 'repeat' ? (
        <RepeatGlyph one={repeatOne === true} size={28} />
      ) : (
        <Icon name={glyph} size={28} />
      )}
    </View>
  );
  /* The nut's clip-path clips every paint of the button itself, so the focus
     plate lives on this square wrapper (design-language §8). */
  if (glyph !== undefined) {
    return control;
  }
  return <View dataSet={{ hexWrap: '1' }}>{control}</View>;
}

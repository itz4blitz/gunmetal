import { useEffect, useState } from 'react';
import { Text, View } from 'react-native-web';
import type { ShellMessages } from '../messages/en/shell.ts';
import { CoverTile } from './destinations/CoverTile.tsx';
import { formatDuration } from './format.ts';
import { noLyrics, type LyricsResolver } from './content.ts';
import { Icon, type IconName } from './Icon.tsx';
import { LyricsPane } from './LyricsPane.tsx';
import type { PlayerQueueLine, PlayerSnapshot } from '../../../ports/src/provisional/player.ts';

export type PlayerPlacement = 'overlay' | 'pane';

export type PlayerFullProps = {
  lyricsFor?: LyricsResolver | undefined;
  messages: ShellMessages;
  playback: PlayerSnapshot;
  open: boolean;
  placement?: PlayerPlacement | undefined;
  albumTitle?: string | undefined;
  volume?: number | undefined;
  onVolume?: ((volume: number) => void) | undefined;
  /** Seek within the playing track; without it the scrubber only shows progress. */
  onSeek?: ((positionMs: number) => void) | undefined;
  onClose: () => void;
  onPlayPause?: (() => void) | undefined;
  onPrevious?: (() => void) | undefined;
  onNext?: (() => void) | undefined;
  onToggleQueue?: (() => void) | undefined;
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
 * The now-playing view (SUR-010). Three regions inside one body: the chrome
 * (heading and collapse), the stage (the artwork, or the lyrics that take its
 * place) and the console (title block, scrubber, transport, secondary row,
 * what plays next). The stylesheet sets the console under the stage on a
 * narrow player and beside it on a wide one.
 */
export function PlayerFull({
  lyricsFor = () => noLyrics,
  messages,
  playback,
  open,
  placement = 'overlay',
  albumTitle,
  volume,
  onVolume,
  onSeek,
  onClose,
  onPlayPause,
  onPrevious,
  onNext,
  onToggleQueue,
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

  if (playback.trackId === undefined || !(open || leaving)) {
    return null;
  }

  const progress = playback.durationMs > 0 ? Math.min(1, playback.positionMs / playback.durationMs) : 0;
  const remainingMs = playback.durationMs > 0 ? Math.max(0, playback.durationMs - playback.positionMs) : 0;
  const upNext = nextPlayerQueueLine(playback);
  const fromLabel = albumTitle !== undefined && albumTitle !== '' ? `${messages.playingFrom} ${albumTitle}` : undefined;
  const lyricsShown = canLyrics && lyricsOpen;
  /* A resolver that has nothing for this track answers with the shared
     placeholder; the pane then shows the quiet empty state. */
  const lyrics = canLyrics ? lyricsFor(playback.trackId, playback.lyricsKind) : noLyrics;
  const seekFromPointer = (event: { currentTarget: HTMLElement; clientX: number }) => {
    const rect = event.currentTarget.getBoundingClientRect();
    if (onSeek === undefined || playback.durationMs <= 0 || rect.width <= 0) {
      return;
    }
    const fraction = Math.max(0, Math.min(1, (event.clientX - rect.left) / rect.width));
    onSeek(Math.round(fraction * playback.durationMs));
  };
  const seekFromKey = (event: { key: string; preventDefault: () => void }) => {
    const step = SEEK_STEPS[event.key];
    if (onSeek === undefined || playback.durationMs <= 0 || step === undefined) {
      return;
    }
    event.preventDefault();
    onSeek(Math.max(0, Math.min(playback.durationMs, playback.positionMs + step)));
  };
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
        dataSet={{
          open: open ? '1' : '0',
          placement,
          queueSheet: queueOpen && queueRaised ? 'over' : 'under',
        }}
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
              <LyricsPane
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
            </View>
            <View id="player-full-progress" dataSet={{ progress: `${Math.round(progress * 100)}` }}>
              <View
                id="player-full-scrubber"
                accessibilityRole="slider"
                accessibilityLabel={messages.progress}
                aria-valuemin={0}
                aria-valuemax={playback.durationMs}
                aria-valuenow={Math.round(playback.positionMs)}
                aria-valuetext={`${formatDuration(playback.positionMs)} of ${formatDuration(playback.durationMs)}`}
                dataSet={{ seekable: seekable ? '1' : '0' }}
                tabIndex={seekable ? 0 : -1}
                onClick={seekFromPointer}
                onKeyDown={seekFromKey}
              >
                <View id="player-full-progress-track">
                  <View
                    id="player-full-progress-fill"
                    dataSet={{ fill: `${Math.round(progress * 100)}` }}
                    style={{ width: `${Math.round(progress * 100)}%` }}
                  />
                </View>
              </View>
              <View id="player-full-time">
                <Text id="player-full-elapsed">{formatDuration(playback.positionMs)}</Text>
                <Text id="player-full-remaining">{formatDuration(remainingMs)}</Text>
              </View>
            </View>
            <View id="player-full-transport">
              <FullControl id="player-full-skip-back" label={messages.previous} onPress={onPrevious} glyph="previous" />
              <FullControl
                id="player-full-play"
                label={playback.playing ? messages.pause : messages.play}
                onPress={onPlayPause}
                playing={playback.playing}
              />
              <FullControl id="player-full-skip-next" label={messages.next} onPress={onNext} glyph="next" />
            </View>
            <View id="player-full-footer">
              {volume === undefined || onVolume === undefined ? null : (
                <View id="player-full-volume" dataSet={{ volume: '1' }}>
                  <View id="player-full-volume-icon">
                    <Icon name="volume" size={20} />
                  </View>
                  <input
                    id="player-full-volume-range"
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

type FullControlProps = {
  id: string;
  label: string;
  onPress?: (() => void) | undefined;
  /** A plain control draws this icon; without one the control is the nut. */
  glyph?: IconName | undefined;
  playing?: boolean | undefined;
};

function FullControl({ id, label, onPress, glyph, playing = false }: FullControlProps) {
  const press = () => {
    onPress?.();
  };
  const control = (
    <View
      id={id}
      dataSet={{
        playerControl: glyph === undefined ? 'primary' : 'plain',
        playing: playing ? '1' : '0',
      }}
      accessibilityRole="button"
      accessibilityLabel={label}
      tabIndex={0}
      onClick={press}
      onKeyDown={onActivate(press)}
    >
      {glyph === undefined ? <Text dataSet={{ controlLabel: '1' }}>{label}</Text> : <Icon name={glyph} size={28} />}
    </View>
  );
  /* The nut's clip-path clips every paint of the button itself, so the focus
     plate lives on this square wrapper (design-language §8). */
  if (glyph !== undefined) {
    return control;
  }
  return <View dataSet={{ hexWrap: '1' }}>{control}</View>;
}

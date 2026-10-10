import { Fragment, useEffect } from 'react';
import { Text, View } from 'react-native-web';
import type { ShellMessages } from '../messages/en/shell.ts';
import { CoverTile } from './destinations/CoverTile.tsx';
import { Icon } from './Icon.tsx';
import { formatDuration } from './format.ts';
import type { PlayerSnapshot } from '../../../ports/src/provisional/player.ts';

export type QueuePaneProps = {
  messages: ShellMessages;
  playback: PlayerSnapshot;
  compactSheet: boolean;
  onCloseSheet?: () => void;
  /** Play this queue line now (wired to the controller's playTrack). */
  onPlayLine?: ((albumId: string, trackId: string) => void) | undefined;
  /** Drop this line from the queue; without it no remove control is drawn. */
  onRemoveLine?: ((trackId: string) => void) | undefined;
};

export function QueuePane({
  messages,
  playback,
  compactSheet,
  onCloseSheet,
  onPlayLine,
  onRemoveLine,
}: QueuePaneProps) {
  /* The open sheet owns Escape: the key closes the queue and is consumed
     here, in the capture phase, before the full player's own Escape handler
     further out can take the view down with it. */
  const queueOpen = playback.queueOpen;
  useEffect(() => {
    if (!compactSheet || !queueOpen) {
      return;
    }
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') {
        return;
      }
      event.preventDefault();
      event.stopImmediatePropagation();
      onCloseSheet?.();
    };
    globalThis.addEventListener('keydown', onKey, true);
    return () => {
      globalThis.removeEventListener('keydown', onKey, true);
    };
  }, [compactSheet, queueOpen, onCloseSheet]);
  const playLine = (line: PlayerSnapshot['queue'][number]) => () => {
    onPlayLine?.(line.albumId, line.trackId);
  };
  const playLineOnKey =
    (line: PlayerSnapshot['queue'][number]) => (event: { key: string; preventDefault: () => void }) => {
      if (event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        onPlayLine?.(line.albumId, line.trackId);
      }
    };
  /* The first line of the playing track carries the "Now playing" label; the
     stylesheet sets it apart from the lines that follow. */
  const playingIndex = playback.queue.findIndex((line) => line.trackId === playback.trackId);
  const body = (
    <View
      id={compactSheet ? 'queue-sheet' : 'right-pane'}
      accessibilityRole="complementary"
      accessibilityLabel={messages.rightPane}
      accessibilityElementsHidden={compactSheet ? !playback.queueOpen : undefined}
      dataSet={compactSheet ? { queueOpen: playback.queueOpen ? '1' : '0' } : undefined}
    >
      <View dataSet={{ queueHeader: '1' }}>
        <Text accessibilityRole="header" dataSet={{ queueHeading: '1' }}>
          {messages.queueHeading}
        </Text>
        {playback.queue.length === 0 ? null : <Text dataSet={{ queueCount: '1' }}>{playback.queue.length}</Text>}
        {compactSheet ? (
          <View
            id="queue-close"
            accessibilityRole="button"
            accessibilityLabel={messages.queueClose}
            tabIndex={playback.queueOpen ? 0 : -1}
            onClick={() => {
              onCloseSheet?.();
            }}
            onKeyDown={(event) => {
              if (event.key === 'Enter' || event.key === ' ') {
                event.preventDefault();
                onCloseSheet?.();
              }
            }}
          >
            <Icon name="close" size={18} />
          </View>
        ) : null}
      </View>
      {playback.queue.length === 0 ? (
        <View dataSet={{ queueEmpty: '1' }}>
          <View dataSet={{ queueEmptyMark: '1' }}>
            <Icon name="queue" size={24} />
          </View>
          <Text id="queue-empty" dataSet={{ emptyState: 'queue' }}>
            {messages.queueEmpty}
          </Text>
        </View>
      ) : (
        <View id="queue-list">
          {playback.queue.map((line, index) => (
            <Fragment key={line.trackId}>
              {index === playingIndex ? <Text dataSet={{ queueLabel: 'now' }}>{messages.playerRegion}</Text> : null}
              <View
                id={`queue-line-${line.trackId}`}
                dataSet={{
                  queueLine: line.trackId,
                  current: line.trackId === playback.trackId ? '1' : '0',
                }}
              >
                <CoverTile
                  tone={line.coverTone}
                  label={line.title}
                  size="row"
                  coverId={`queue-art-${line.trackId}`}
                  artUrl={line.coverUrl}
                />
                <View dataSet={{ nowPlaying: line.trackId === playback.trackId && playback.playing ? '1' : '0' }} />
                <Text dataSet={{ queueTitle: '1' }}>{line.title}</Text>
                <Text dataSet={{ queueArtist: '1' }}>{line.artistName}</Text>
                <Text dataSet={{ queueDuration: '1' }}>{formatDuration(line.durationMs)}</Text>
                <View
                  id={`queue-play-${line.trackId}`}
                  dataSet={{ queuePlay: '1' }}
                  accessibilityRole="button"
                  accessibilityLabel={`${messages.play} ${line.title}`}
                  tabIndex={0}
                  onClick={playLine(line)}
                  onKeyDown={playLineOnKey(line)}
                >
                  <Text dataSet={{ controlLabel: '1' }}>{messages.play}</Text>
                </View>
                {onRemoveLine === undefined ? null : (
                  <View
                    id={`queue-remove-${line.trackId}`}
                    dataSet={{ queueRemove: '1' }}
                    accessibilityRole="button"
                    accessibilityLabel={`${messages.playerRemove}: ${line.title}`}
                    tabIndex={0}
                    onClick={() => {
                      onRemoveLine(line.trackId);
                    }}
                    onKeyDown={(event) => {
                      if (event.key === 'Enter' || event.key === ' ') {
                        event.preventDefault();
                        onRemoveLine(line.trackId);
                      }
                    }}
                  >
                    <Icon name="close" size={16} />
                  </View>
                )}
              </View>
            </Fragment>
          ))}
        </View>
      )}
    </View>
  );

  if (!compactSheet) {
    return body;
  }
  return (
    <View dataSet={{ queueLayer: '1' }}>
      {playback.queueOpen ? (
        <View
          id="queue-scrim"
          onClick={() => {
            onCloseSheet?.();
          }}
        />
      ) : null}
      {body}
    </View>
  );
}

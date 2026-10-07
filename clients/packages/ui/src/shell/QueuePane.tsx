import { Text, View } from 'react-native-web';
import type { ShellMessages } from '../messages/en/shell.ts';
import { CoverTile } from './destinations/CoverTile.tsx';
import { formatDuration } from './format.ts';
import type { PlayerSnapshot } from '../../../ports/src/provisional/player.ts';

export type QueuePaneProps = {
  messages: ShellMessages;
  playback: PlayerSnapshot;
  compactSheet: boolean;
  onCloseSheet?: () => void;
  /** Play this queue line now (wired to the controller's playTrack). */
  onPlayLine?: ((albumId: string, trackId: string) => void) | undefined;
};

export function QueuePane({ messages, playback, compactSheet, onCloseSheet, onPlayLine }: QueuePaneProps) {
  const closeOnKey = (event: { key: string; preventDefault: () => void }) => {
    if (event.key === 'Escape') {
      event.preventDefault();
      onCloseSheet?.();
    }
  };
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
  const body = (
    <View
      id={compactSheet ? 'queue-sheet' : 'right-pane'}
      accessibilityRole="complementary"
      accessibilityLabel={messages.rightPane}
      accessibilityElementsHidden={compactSheet ? !playback.queueOpen : undefined}
      dataSet={compactSheet ? { queueOpen: playback.queueOpen ? '1' : '0' } : undefined}
      onKeyDown={compactSheet ? closeOnKey : undefined}
    >
      <View dataSet={{ queueHeader: '1' }}>
        <Text accessibilityRole="header" dataSet={{ queueHeading: '1' }}>
          {messages.queueHeading}
        </Text>
        {playback.queue.length === 0 ? null : <Text dataSet={{ queueCount: '1' }}>{playback.queue.length}</Text>}
        {compactSheet ? (
          <View
            id="queue-sheet-close"
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
            <Text>{messages.queueClose}</Text>
          </View>
        ) : null}
      </View>
      {playback.queue.length === 0 ? (
        <View dataSet={{ emptyCard: '1', emptyRow: '1' }}>
          <View dataSet={{ emptyMark: '1' }} />
          <Text dataSet={{ emptyTitle: '1' }}>{messages.queue}</Text>
          <Text id="queue-empty" dataSet={{ emptyState: 'queue' }}>
            {messages.queueEmpty}
          </Text>
        </View>
      ) : (
        <View id="queue-list">
          {playback.queue.map((line) => (
            <View
              key={line.trackId}
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
            </View>
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

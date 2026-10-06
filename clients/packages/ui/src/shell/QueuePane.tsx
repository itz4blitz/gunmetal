import { Text, View } from 'react-native-web';
import type { ShellMessages } from '../messages/en/shell.ts';
import { formatDuration } from './format.ts';
import type { PlaybackSnapshot } from './playback.ts';

export type QueuePaneProps = {
  messages: ShellMessages;
  playback: PlaybackSnapshot;
  compactSheet: boolean;
  onCloseSheet?: () => void;
};

export function QueuePane({ messages, playback, compactSheet, onCloseSheet }: QueuePaneProps) {
  if (compactSheet && !playback.queueOpen) {
    return null;
  }

  const body = (
    <View
      id={compactSheet ? 'queue-sheet' : 'right-pane'}
      accessibilityRole="complementary"
      accessibilityLabel={messages.rightPane}
    >
      <View dataSet={{ queueHeader: '1' }}>
        <Text accessibilityRole="header">{messages.queueHeading}</Text>
        {compactSheet ? (
          <View
            id="queue-sheet-close"
            accessibilityRole="button"
            accessibilityLabel={messages.queueClose}
            tabIndex={0}
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
            <Text>{messages.queue}</Text>
          </View>
        ) : null}
      </View>
      {playback.queue.length === 0 ? (
        <Text id="queue-empty">{messages.queueEmpty}</Text>
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
              <Text dataSet={{ queueTitle: '1' }}>{line.title}</Text>
              <Text dataSet={{ queueArtist: '1' }}>{line.artistName}</Text>
              <Text dataSet={{ queueDuration: '1' }}>{formatDuration(line.durationMs)}</Text>
            </View>
          ))}
        </View>
      )}
    </View>
  );

  return body;
}

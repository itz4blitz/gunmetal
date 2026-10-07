import { Text, View } from 'react-native-web';

export type LyricsPaneProps = {
  id: string;
  label: string;
  lines: readonly string[];
  synced: boolean;
  open: boolean;
};

export function LyricsPane({ id, label, lines, synced, open }: LyricsPaneProps) {
  if (!open) {
    return null;
  }

  return (
    <View
      id={id}
      accessibilityRole="region"
      accessibilityLabel={label}
      dataSet={{ lyricsPane: '1', synced: synced ? '1' : '0' }}
    >
      {lines.map((line, index) => (
        <Text
          key={`${index}:${line}`}
          dataSet={{
            lyricsLine: '1',
            current: synced && index === 0 ? '1' : '0',
          }}
        >
          {line}
        </Text>
      ))}
    </View>
  );
}

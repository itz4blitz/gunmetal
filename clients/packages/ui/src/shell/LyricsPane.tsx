import { Text, View } from 'react-native-web';

/**
 * Distance from the current line beyond which the falloff saturates: the CSS
 * ramp in apps/demo/public/area-lyrics.css styles exactly these steps, and
 * the value is always a small integer, never free text.
 */
const FAR_DISTANCE = 4;

export type LyricsPaneProps = {
  id: string;
  label: string;
  lines: readonly string[];
  synced: boolean;
  open: boolean;
  /** Synced only: index of the highlighted line. Defaults to 0, the static demo's first line. */
  currentLine?: number | undefined;
  /** True when the track has no lyrics (the resolver returned the placeholder): quiet centred chrome. */
  empty?: boolean | undefined;
};

export function LyricsPane({ id, label, lines, synced, open, currentLine = 0, empty = false }: LyricsPaneProps) {
  if (!open) {
    return null;
  }

  return (
    <View
      id={id}
      accessibilityRole="region"
      accessibilityLabel={label}
      dataSet={{ lyricsPane: '1', synced: synced ? '1' : '0', empty: empty ? '1' : '0' }}
    >
      {lines.map((line, index) => {
        const distance = synced ? Math.min(Math.abs(index - currentLine), FAR_DISTANCE) : 0;
        return (
          <Text
            key={`${index}:${line}`}
            dataSet={{
              lyricsLine: '1',
              current: synced && index === currentLine ? '1' : '0',
              distance: `${distance}`,
            }}
          >
            {line}
          </Text>
        );
      })}
    </View>
  );
}

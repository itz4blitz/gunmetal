import { Text, View } from 'react-native-web';
import { tokenClass, type ShellProps } from './compose.ts';

// One view and one line of text with a token colour, plus the value that changes at run time.
export function Shell({ label, count }: ShellProps) {
  return (
    <View id={tokenClass()}>
      <Text>{label}</Text>
      <Text accessibilityRole="status" accessibilityLabel={label}>
        {String(count)}
      </Text>
    </View>
  );
}

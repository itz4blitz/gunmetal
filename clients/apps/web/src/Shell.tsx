import { Text, View } from 'react-native-web';
import { tokenClass, type ShellProps } from './compose.ts';

export function Shell({ wordmark, title, hint, clock }: ShellProps) {
  return (
    <View id={tokenClass()}>
      <View id="shell-header">
        <Text id="shell-wordmark">{wordmark}</Text>
        <Text id="shell-clock" accessibilityRole="status" accessibilityLabel="Uptime">
          {clock}
        </Text>
      </View>
      <View id="shell-main">
        <Text id="shell-title">{title}</Text>
        <Text id="shell-hint">{hint}</Text>
      </View>
      <View id="shell-bar">
        <View id="shell-play" />
      </View>
    </View>
  );
}

import { Text, View } from 'react-native-web';

export type PlayerBarProps = {
  label: string;
  emptyLabel: string;
};

export function PlayerBar({ label, emptyLabel }: PlayerBarProps) {
  return (
    <View id="player-bar" accessibilityRole="region" accessibilityLabel={label}>
      <View id="shell-play" />
      <Text id="player-empty">{emptyLabel}</Text>
    </View>
  );
}

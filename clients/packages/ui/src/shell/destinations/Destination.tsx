import { Text, View } from 'react-native-web';

export type DestinationProps = {
  headline: string;
};

export function Destination({ headline }: DestinationProps) {
  return (
    <View id="destination">
      <Text id="destination-headline" accessibilityRole="header">
        {headline}
      </Text>
    </View>
  );
}

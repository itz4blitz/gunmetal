import { Text, View } from 'react-native-web';

export type CoverTileProps = {
  tone: string;
  label: string;
  size?: 'row' | 'grid' | 'detail' | 'bar';
  coverId?: string;
};

export function CoverTile({ tone, label, size = 'grid', coverId }: CoverTileProps) {
  const trimmed = label.trim();
  const glyph = trimmed.length === 0 ? '·' : trimmed.slice(0, 1).toUpperCase();
  return (
    <View
      id={coverId ?? `cover-${size}`}
      dataSet={{ cover: tone, size }}
      accessibilityLabel={label}
      accessibilityRole="image"
    >
      <View dataSet={{ coverPlate: '1' }} />
      <View dataSet={{ coverWash: '1' }} />
      <View dataSet={{ coverSheen: '1' }} />
      <Text dataSet={{ coverLabel: '1' }}>{glyph}</Text>
    </View>
  );
}

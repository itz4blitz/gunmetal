import { Text, View } from 'react-native-web';

/** An artwork URL from the library (same-origin); the tone plate is the base. */
export type CoverTileProps = {
  tone: string;
  label: string;
  size?: 'row' | 'grid' | 'detail' | 'bar' | 'full';
  coverId?: string;
  artUrl?: string | undefined;
};

/**
 * Artwork placeholder until real covers sync (LIB-142): a quiet tone plate,
 * the hexagon outline from the brand mark, and — only where the artwork is
 * too small to read a title next to it — the initial. No letter posters.
 */
export function CoverTile({ tone, label, size = 'grid', coverId, artUrl }: CoverTileProps) {
  const art =
    artUrl === undefined || artUrl === ''
      ? undefined
      : { backgroundImage: `url("${artUrl}")`, backgroundSize: 'cover', backgroundPosition: 'center' };
  const showGlyph = (size === 'bar' || size === 'detail' || size === 'full') && (artUrl === undefined || artUrl === '');
  const trimmed = label.trim();
  const glyph = trimmed.length === 0 ? '·' : trimmed.slice(0, 1).toUpperCase();
  return (
    <View
      id={coverId ?? `cover-${size}`}
      dataSet={{ cover: tone, size }}
      accessibilityLabel={label}
      accessibilityRole="image"
      style={art}
    >
      <View dataSet={{ coverPlate: '1' }} />
      <View dataSet={{ coverWash: '1' }} />
      {showGlyph ? <Text dataSet={{ coverLabel: '1' }}>{glyph}</Text> : null}
    </View>
  );
}

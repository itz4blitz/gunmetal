import { Text, View } from 'react-native-web';
import { useNearById, type NearViewFactory } from './near-view.ts';

/** An artwork URL from the library (same-origin); the tone plate is the base. */
export type CoverTileProps = {
  tone: string;
  label: string;
  size?: 'row' | 'grid' | 'detail' | 'bar' | 'full';
  coverId?: string;
  artUrl?: string | undefined;
  /**
   * When wired, the art URL waits until the tile is near the viewport (the
   * library grids pass a factory over IntersectionObserver). Without it —
   * jsdom, or a host without the API — the art paints immediately.
   */
  deferArt?: NearViewFactory | undefined;
};

/**
 * Artwork placeholder until real covers sync (LIB-142): a quiet tone plate,
 * the hexagon outline from the brand mark, and — only where the artwork is
 * too small to read a title next to it — the initial. No letter posters.
 *
 * Art-first: when a real cover is present the plate and wash stand down
 * (data-cover-art) and the machined edge carries the tile; the plate stays
 * the loading base underneath.
 */
export function CoverTile({ tone, label, size = 'grid', coverId, artUrl, deferArt }: CoverTileProps) {
  const id = coverId ?? `cover-${size}`;
  const hasArt = artUrl !== undefined && artUrl !== '';
  const near = useNearById(hasArt ? deferArt : undefined, id);
  const art =
    hasArt && near
      ? { backgroundImage: `url("${artUrl}")`, backgroundSize: 'cover', backgroundPosition: 'center' }
      : undefined;
  const showGlyph = !hasArt && (size === 'bar' || size === 'detail' || size === 'full');
  const trimmed = label.trim();
  const glyph = trimmed.length === 0 ? '·' : trimmed.slice(0, 1).toUpperCase();
  return (
    <View
      id={id}
      dataSet={{ cover: tone, size, coverArt: hasArt ? '1' : '0' }}
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

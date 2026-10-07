/**
 * The repeat glyphs the transport needs, drawn in the shell's icon grammar
 * (one stroked path on the 24px grid, currentColor, hidden from assistive
 * tech because the control around the glyph is named). The shared set in
 * Icon.tsx has no repeat mark yet; these live beside it until the sets
 * merge, so the transport does not wait on another surface's deed.
 */

const REPEAT_PATH = 'M17 1l4 4-4 4 M3 11V9a4 4 0 0 1 4-4h14 M7 23l-4-4 4-4 M21 13v2a4 4 0 0 1-4 4H3';

/** The stem and flag of the "one" that marks repeat one. */
const ONE_PATH = 'M10.6 11.2 12 10.4v6.2';

export type RepeatGlyphProps = {
  /** Repeat one draws the mark that says the track itself comes round again. */
  one?: boolean | undefined;
  /** Rendered edge in CSS px (18 in the bar, 20 in the full player). */
  size?: number | undefined;
};

export function RepeatGlyph({ one = false, size = 20 }: RepeatGlyphProps) {
  return (
    <svg
      data-icon={one ? 'repeat-one' : 'repeat'}
      viewBox="0 0 24 24"
      width={size}
      height={size}
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d={REPEAT_PATH} />
      {one ? <path d={ONE_PATH} /> : null}
    </svg>
  );
}

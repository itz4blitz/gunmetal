/**
 * The Gunmetal mark (design-language §1): a hexagonal steel nut lit from
 * the top left, with a brass play triangle sitting in its bore — the same
 * identity as docs/icon.svg, drawn inline so no image is fetched and the
 * CSP stays clean. Decorative: the wordmark beside it carries the name.
 */
export function BrandMark({ size = 22 }: { size?: number | undefined }) {
  return (
    <svg data-brand-mark="1" viewBox="0 0 24 24" width={size} height={size} aria-hidden="true" focusable="false">
      <defs>
        <linearGradient id="gm-nut-face" x1="0.15" y1="0" x2="0.85" y2="1">
          <stop offset="0" stopColor="#55626f" />
          <stop offset="0.45" stopColor="#2e3842" />
          <stop offset="1" stopColor="#151a1f" />
        </linearGradient>
        <linearGradient id="gm-nut-edge" x1="0.1" y1="0" x2="0.9" y2="1">
          <stop offset="0" stopColor="#e8f1f8" stopOpacity="0.85" />
          <stop offset="0.5" stopColor="#e8f1f8" stopOpacity="0.15" />
          <stop offset="1" stopColor="#e8f1f8" stopOpacity="0" />
        </linearGradient>
        <linearGradient id="gm-nut-play" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#f3cd77" />
          <stop offset="1" stopColor="#d4952f" />
        </linearGradient>
      </defs>
      {/* The nut: a pointy-top hexagon with a machined top-left edge. */}
      <path
        data-nut-face="1"
        d="M12 2.2l8.5 4.9v9.8L12 21.8 3.5 16.9V7.1z"
        fill="url(#gm-nut-face)"
        stroke="url(#gm-nut-edge)"
        strokeWidth="0.9"
      />
      {/* The bore. */}
      <circle cx="12" cy="12" r="4.6" fill="#10151a" stroke="#0b0f13" strokeWidth="0.6" />
      {/* The brass play sitting in the bore. */}
      <path data-nut-play="1" d="M10.7 9.6l4.4 2.4-4.4 2.4z" fill="url(#gm-nut-play)" />
    </svg>
  );
}

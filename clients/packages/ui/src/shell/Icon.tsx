/**
 * The shell's icon set: inline SVG elements on the 24px grid with round caps
 * and joins (design-language §10). Each icon is one stroked path compiled
 * into the bundle — no style inside, no image asset, no icon font — and
 * follows `currentColor`, so a theme or a state recolours it for free.
 */
const ICON_PATHS = {
  home: 'M3 10.6 12 3l9 7.6V20a1 1 0 0 1-1 1h-5v-6.5H9V21H4a1 1 0 0 1-1-1z',
  search: 'M17.5 10.5a7 7 0 1 1-14 0 7 7 0 0 1 14 0z M20.5 20.5l-4.6-4.6',
  library: 'M4 4v16 M8.5 4v16 M13 5.2l4.6 14.6 M20 20H4',
  store: 'M4 4.5h16v15H4z M12 4.5v15 M4 12h16',
  watch: 'M4 5h16a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1z M8 20.5h8 M10 8.5v5l4.5-2.5z',
  settings: 'M4 7h9 M17 7h3 M4 17h3 M11 17h9 M15 4.5v5 M9 14.5v5',
  previous: 'M19 5.5v13L9 12z M5.5 5v14',
  next: 'M5 5.5v13L15 12z M18.5 5v14',
  shuffle:
    'M3 17h2.2a4 4 0 0 0 3.3-1.7l5-7.6A4 4 0 0 1 16.8 6H21 M18 3l3 3-3 3 M3 7h2.2a4 4 0 0 1 3 1.4 M21 18h-4.2a4 4 0 0 1-3-1.4 M18 15l3 3-3 3',
  queue: 'M4 6h16 M4 11h16 M4 16h8 M18 14v6 M15 17h6',
  lyrics: 'M5 4.5h14a1 1 0 0 1 1 1V15a1 1 0 0 1-1 1h-7l-4.5 4v-4H5a1 1 0 0 1-1-1V5.5a1 1 0 0 1 1-1z M8 8.5h8 M8 12h5',
  expand: 'M15 4h5v5 M9 20H4v-5 M20 4l-6.5 6.5 M4 20l6.5-6.5',
  collapse: 'M6 9l6 6 6-6',
  volume: 'M4 9.5v5h3.5l5 4v-13l-5 4z M16 9a4.2 4.2 0 0 1 0 6 M18.6 6.5a8 8 0 0 1 0 11',
  mute: 'M4 9.5v5h3.5l5 4v-13l-5 4z M16.5 9.5l5 5 M21.5 9.5l-5 5',
  back: 'M15 5l-7 7 7 7',
  more: 'M12 5.5v.01 M12 12v.01 M12 18.5v.01',
  close: 'M6 6l12 12 M18 6 6 18',
  check: 'M5 12.5l4.5 4.5L19 7.5',
  pin: 'M14.2 3.8 20.2 9.8 15.6 11.2 11.2 15.6 9.8 20.2 3.8 14.2 8.4 12.8 12.8 8.4z M8.6 15.4 4.8 19.2',
  rows: 'M4 7h16 M4 12h16 M4 17h16',
  rowsDense: 'M4 5.5h16 M4 10h16 M4 14.5h16 M4 19h16',
  clock: 'M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0z M12 7v5l3.2 2',
  disc: 'M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0z M14.5 12a2.5 2.5 0 1 1-5 0 2.5 2.5 0 0 1 5 0z',
} as const;

export type IconName = keyof typeof ICON_PATHS;

export type IconProps = {
  name: IconName;
  /** Accessible name; omit when the control around the icon already has one. */
  label?: string | undefined;
  /** Rendered edge in CSS px (16 inline, 20 dense rows, 24 default). */
  size?: number | undefined;
};

export function Icon({ name, label, size = 20 }: IconProps) {
  const naming =
    label === undefined ? ({ 'aria-hidden': true } as const) : ({ 'aria-label': label, role: 'img' } as const);
  return (
    <svg
      data-icon={name}
      viewBox="0 0 24 24"
      width={size}
      height={size}
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      strokeLinejoin="round"
      {...naming}
    >
      <path d={ICON_PATHS[name]} />
    </svg>
  );
}

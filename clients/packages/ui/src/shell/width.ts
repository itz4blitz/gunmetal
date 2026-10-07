export type WidthClass = 'compact' | 'medium' | 'expanded' | 'wide';

// Breakpoints from design-language.md ("Breakpoints and density").
export function widthClass(widthPx: number): WidthClass {
  if (widthPx < 600) {
    return 'compact';
  }
  if (widthPx < 1024) {
    return 'medium';
  }
  if (widthPx < 1440) {
    return 'expanded';
  }
  return 'wide';
}

export function landmarksForClass(width: WidthClass): readonly string[] {
  if (width === 'compact') {
    return ['content', 'player-bar', 'nav-tabs'];
  }
  if (width === 'medium') {
    return ['nav-rail', 'content', 'player-bar'];
  }
  if (width === 'expanded') {
    return ['nav-sidebar', 'content', 'player-bar'];
  }
  return ['nav-sidebar', 'content', 'right-pane', 'player-bar'];
}

// Landmark ids present at each width, in document order (CP-009).
export function landmarks(widthPx: number): readonly string[] {
  return landmarksForClass(widthClass(widthPx));
}

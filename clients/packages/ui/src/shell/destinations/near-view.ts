import { useEffect, useState } from 'react';

/**
 * Near-viewport art deferral for the CSS-background-image covers: a factory
 * over the browser's IntersectionObserver (injectable, so tests and jsdom —
 * which has no IntersectionObserver — stay eager) and the hook that withholds
 * an element's art until its factory reports it near the viewport.
 */

/** Attaches near-viewport detection to a target; answers the disposer. */
export type NearViewFactory = (target: Element, onNear: () => void) => () => void;

/**
 * A factory over a real IntersectionObserver. Without the constructor —
 * jsdom, or a host without the API — there is no factory and every surface
 * renders its art immediately.
 */
export function intersectionObserverFactory(
  ObserverCtor: typeof IntersectionObserver | undefined,
  rootMargin = '200px',
): NearViewFactory | undefined {
  if (ObserverCtor === undefined) {
    return undefined;
  }
  return (target, onNear) => {
    const observer = new ObserverCtor(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          onNear();
        }
      },
      { rootMargin },
    );
    observer.observe(target);
    return () => {
      observer.disconnect();
    };
  };
}

/**
 * Whether the element with `elementId` may paint its art. Without a factory
 * the answer is always yes; with one it turns yes at the first intersecting
 * report. A missing element stays no — it cannot promise what it cannot see.
 */
export function useNearById(factory: NearViewFactory | undefined, elementId: string): boolean {
  const [near, setNear] = useState(factory === undefined);
  useEffect(() => {
    if (factory === undefined) {
      return undefined;
    }
    const target = document.getElementById(elementId);
    if (target === null) {
      return undefined;
    }
    return factory(target, () => {
      setNear(true);
    });
  }, [factory, elementId]);
  return near;
}

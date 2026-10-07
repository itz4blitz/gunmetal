import { act } from 'react';
import { cleanup, renderHook } from '@testing-library/react';
import { afterEach, describe, expect, test, vi } from 'vitest';
import { intersectionObserverFactory, useNearById } from './near-view.ts';

afterEach(cleanup);

/** A scriptable stand-in for the browser's IntersectionObserver. */
class StubObserver {
  static instances: StubObserver[] = [];
  readonly observed: Element[] = [];
  disconnected = false;
  readonly callback: (entries: Array<{ isIntersecting: boolean }>) => void;
  readonly options: Record<string, unknown> | undefined;
  constructor(callback: (entries: Array<{ isIntersecting: boolean }>) => void, options?: Record<string, unknown>) {
    this.callback = callback;
    this.options = options;
    StubObserver.instances = [];
    StubObserver.instances.push(this);
  }
  observe(target: Element): void {
    this.observed.push(target);
  }
  unobserve(): void {}
  disconnect(): void {
    this.disconnected = true;
  }
  see(isIntersecting: boolean): void {
    this.callback([{ isIntersecting }]);
  }
}

const asIO = StubObserver as unknown as typeof IntersectionObserver;

describe('intersectionObserverFactory', () => {
  test('without an IntersectionObserver there is no factory (jsdom renders eagerly)', () => {
    expect(intersectionObserverFactory(undefined)).toBeUndefined();
  });

  test('the factory observes the target and answers a disposer', () => {
    const factory = intersectionObserverFactory(asIO);
    if (factory === undefined) {
      throw new Error('stub observer missing');
    }
    const target = document.createElement('div');
    const onNear = vi.fn();
    const dispose = factory(target, onNear);
    expect(StubObserver.instances[0]?.observed).toStrictEqual([target]);
    // Rooted near the viewport, not at the exact edge.
    expect(StubObserver.instances[0]?.options).toStrictEqual({ rootMargin: '200px' });
    expect(onNear).not.toHaveBeenCalled();
    act(() => {
      StubObserver.instances[0]?.see(true);
    });
    expect(onNear).toHaveBeenCalledTimes(1);
    // A non-intersecting report is not "near".
    act(() => {
      StubObserver.instances[0]?.see(false);
    });
    expect(onNear).toHaveBeenCalledTimes(1);
    dispose();
    expect(StubObserver.instances[0]?.disconnected).toStrictEqual(true);
  });
});

describe('useNearById', () => {
  test('without a factory the element is near from the first render', () => {
    const { result } = renderHook(() => useNearById(undefined, 'any-element'));
    expect(result.current).toStrictEqual(true);
  });

  test('with a factory the element waits for its intersection report', () => {
    const element = document.createElement('div');
    element.id = 'near-target';
    document.body.appendChild(element);
    const factory = intersectionObserverFactory(asIO);
    if (factory === undefined) {
      throw new Error('stub observer missing');
    }
    const { result, unmount } = renderHook(() => useNearById(factory, 'near-target'));
    expect(result.current).toStrictEqual(false);
    act(() => {
      StubObserver.instances[0]?.see(true);
    });
    expect(result.current).toStrictEqual(true);
    // The observation is cleaned up with the hook.
    unmount();
    expect(StubObserver.instances[0]?.disconnected).toStrictEqual(true);
  });

  test('a missing element never turns near (it simply cannot promise it)', () => {
    const factory = intersectionObserverFactory(asIO);
    if (factory === undefined) {
      throw new Error('stub observer missing');
    }
    const { result } = renderHook(() => useNearById(factory, 'no-such-element'));
    expect(result.current).toStrictEqual(false);
  });
});

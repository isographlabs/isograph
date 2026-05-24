import type { ItemCleanupPair } from '@isograph/disposable-types';
import { configure, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import { ParentCache } from './ParentCache';
import { useLazyDisposableState } from './useLazyDisposableState';
configure({ reactStrictMode: true });

function createCache<T>(value: T) {
  const disposeItem = vi.fn();
  const factory = vi.fn(() => {
    const pair: ItemCleanupPair<T> = [value, disposeItem];
    return pair;
  });
  const cache = new ParentCache(factory);
  return { cache, disposeItem };
}

function promiseWithResolvers() {
  let resolve;
  let reject;
  const promise = new Promise((_resolve, _reject) => {
    resolve = _resolve;
    reject = _reject;
  });
  return { resolve, reject, promise };
}

describe('useLazyDisposableState', async () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.restoreAllMocks();
  });

  test('on render it should read the cache', async () => {
    const cache1 = createCache(1);

    const { result } = renderHook(
      (props) => useLazyDisposableState(props.parentCache),
      {
        initialProps: {
          parentCache: cache1.cache,
        },
      },
    );

    expect(result.current.state).toEqual(1);
    expect(cache1.cache.factory).toHaveBeenCalledOnce();
  });

  test('on unmount it should dispose the cache', async () => {
    const cache1 = createCache(1);

    const { unmount } = renderHook(
      (props) => useLazyDisposableState(props.parentCache),
      {
        initialProps: {
          parentCache: cache1.cache,
        },
      },
    );

    unmount();
    vi.runAllTimers();
    expect(cache1.disposeItem).toHaveBeenCalledOnce();
  });

  test('on cache change, it should read new cache', async () => {
    const cache1 = createCache(1);
    const cache2 = createCache(2);

    const { result, rerender, unmount } = renderHook(
      (props) => useLazyDisposableState(props.parentCache),
      {
        initialProps: {
          parentCache: cache1.cache,
        },
      },
    );

    rerender({
      parentCache: cache2.cache,
    });

    expect(result.current.state).toEqual(2);
    expect(cache2.cache.factory).toHaveBeenCalledOnce();
  });

  test('on cache change, it should dispose previous cache', async () => {
    const cache1 = createCache(1);
    const cache2 = createCache(2);

    const { rerender } = renderHook(
      (props) => useLazyDisposableState(props.parentCache),
      {
        initialProps: {
          parentCache: cache1.cache,
        },
      },
    );

    rerender({
      parentCache: cache2.cache,
    });

    vi.runAllTimers();
    expect(cache1.disposeItem).toHaveBeenCalledOnce();
  });
});

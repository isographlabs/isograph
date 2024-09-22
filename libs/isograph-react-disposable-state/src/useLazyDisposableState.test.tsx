import { ItemCleanupPair } from '@isograph/disposable-types';
import React, { useEffect, useState } from 'react';
import { create } from 'react-test-renderer';
import { describe, expect, test, vi } from 'vitest';
import { ParentCache } from './ParentCache';
import { useLazyDisposableState } from './useLazyDisposableState';
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
  test('on cache change, it should dispose previous cache', async () => {
    const cache1 = createCache(1);
    const cache2 = createCache(2);
    const renders = vi.fn();

    let unmounted = promiseWithResolvers();
    let committed = promiseWithResolvers();

    function TestComponent() {
      const [cache, setCache] = useState(cache1.cache);
      const { state } = useLazyDisposableState(cache);

      useEffect(() => {
        setCache(cache2.cache);

        return () => {
          unmounted.resolve();
        };
      }, []);

      useEffect(() => {
        if (state == 1) return;
        committed.resolve();
      }, [state]);

      renders(state);

      return null;
    }

    const root = create(<TestComponent />, { unstable_isConcurrent: true });
    await committed.promise;
    expect(cache1.disposeItem).toHaveBeenCalled();
    expect(cache1.cache.factory).toHaveBeenCalledOnce();
    root.unmount();
    await unmounted.promise;
    expect(cache2.disposeItem).toHaveBeenCalled();
    expect(cache2.cache.factory).toHaveBeenCalledOnce();
    expect(renders).toHaveBeenNthCalledWith(1, 1);
    expect(renders).toHaveBeenNthCalledWith(2, 2);
    expect(renders).toHaveBeenCalledTimes(2);
  });
});

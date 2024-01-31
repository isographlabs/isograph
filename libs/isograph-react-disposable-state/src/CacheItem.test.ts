import {
  describe,
  assert,
  test,
  vi,
  beforeEach,
  afterEach,
  expect,
} from 'vitest';
import {
  CacheItem,
  CacheItemState,
  createTemporarilyRetainedCacheItem,
} from './CacheItem';
import { ItemCleanupPair } from '@isograph/disposable-types';

function getState<T>(cacheItem: CacheItem<T>): CacheItemState<T> {
  return (cacheItem as any).__state as CacheItemState<T>;
}

describe('CacheItem', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  describe('Item that is temporarily retained once', () => {
    test('Temporarily retained cache item gets created in the expected state', () => {
      const removeFromParentCache = vi.fn();
      const disposeItem = vi.fn();
      const factory = vi.fn(() => {
        const ret: ItemCleanupPair<number> = [1, disposeItem];
        return ret;
      });
      const [cacheItem, _disposeTemporaryRetain] =
        createTemporarilyRetainedCacheItem<number>(
          factory,
          removeFromParentCache,
        );

      expect(factory.mock.calls.length).toEqual(1);

      const state = getState(cacheItem);
      assert(state.kind === 'InParentCacheAndNotDisposed');
      expect(state.value).toEqual(1);
      expect(state.permanentRetainCount).toEqual(0);
      expect(state.temporaryRetainCount).toEqual(1);
      expect(state.disposeValue).toEqual(disposeItem);
      expect(state.removeFromParentCache).toEqual(removeFromParentCache);
      expect(disposeItem.mock.calls.length).toEqual(0);
      expect(removeFromParentCache.mock.calls.length).toEqual(0);
    });

    test('Disposal of temporarily retained cache item', () => {
      const removeFromParentCache = vi.fn();
      const disposeItem = vi.fn();
      const factory = vi.fn(() => {
        const ret: ItemCleanupPair<number> = [1, disposeItem];
        return ret;
      });
      const [cacheItem, disposeTemporaryRetain] =
        createTemporarilyRetainedCacheItem<number>(
          factory,
          removeFromParentCache,
        );

      expect(factory.mock.calls.length).toEqual(1);

      disposeTemporaryRetain();

      const state = getState(cacheItem);

      expect(state.kind).toEqual('NotInParentCacheAndDisposed');
      expect(removeFromParentCache.mock.calls.length).toEqual(1);
      expect(disposeItem.mock.calls.length).toEqual(1);
    });

    test('Expiration of temporarily retained cache item', () => {
      const removeFromParentCache = vi.fn();
      const disposeItem = vi.fn();

      const factory = vi.fn(() => {
        const ret: ItemCleanupPair<number> = [1, disposeItem];
        return ret;
      });
      const [cacheItem, _disposeTemporaryRetain] =
        createTemporarilyRetainedCacheItem<number>(
          factory,
          removeFromParentCache,
        );

      expect(factory.mock.calls.length).toEqual(1);

      vi.advanceTimersToNextTimer();

      const state = getState(cacheItem);

      assert(state.kind === 'NotInParentCacheAndDisposed');
      expect(removeFromParentCache.mock.calls.length).toEqual(1);
      expect(disposeItem.mock.calls.length).toEqual(1);
    });

    test('Disposal of expired temporarily retained cache item', () => {
      const removeFromParentCache = vi.fn();
      const disposeItem = vi.fn();

      const factory = vi.fn(() => {
        const ret: ItemCleanupPair<number> = [1, disposeItem];
        return ret;
      });
      const [cacheItem, disposeTemporaryRetain] =
        createTemporarilyRetainedCacheItem<number>(
          factory,
          removeFromParentCache,
        );

      expect(factory.mock.calls.length).toEqual(1);

      vi.advanceTimersToNextTimer();

      const state = getState(cacheItem);

      assert(state.kind === 'NotInParentCacheAndDisposed');
      expect(removeFromParentCache.mock.calls.length).toEqual(1);
      expect(disposeItem.mock.calls.length).toEqual(1);

      expect(() => {
        disposeTemporaryRetain();
      }).not.toThrow();
    });

    test('Repeated disposal of temporarily retained cache item', () => {
      const removeFromParentCache = vi.fn();
      const disposeItem = vi.fn();

      const factory = vi.fn(() => {
        const ret: ItemCleanupPair<number> = [1, disposeItem];
        return ret;
      });
      const [cacheItem, disposeTemporaryRetain] =
        createTemporarilyRetainedCacheItem<number>(
          factory,
          removeFromParentCache,
        );

      expect(factory.mock.calls.length).toEqual(1);

      disposeTemporaryRetain();
      const state = getState(cacheItem);

      assert(state.kind === 'NotInParentCacheAndDisposed');
      expect(removeFromParentCache.mock.calls.length).toEqual(1);
      expect(disposeItem.mock.calls.length).toEqual(1);

      expect(() => {
        disposeTemporaryRetain();
      }).toThrow();
    });
  });

  describe('Item that is temporarily retained once and then permanently retained', () => {
    test('Permanent retain removes the item from the parent', () => {
      const removeFromParentCache = vi.fn();
      const disposeItem = vi.fn();

      const factory = vi.fn(() => {
        const ret: ItemCleanupPair<number> = [1, disposeItem];
        return ret;
      });
      const [cacheItem, disposeTemporaryRetain] =
        createTemporarilyRetainedCacheItem<number>(
          factory,
          removeFromParentCache,
        );

      const mockedDisposeTemporaryRetain = vi.fn(disposeTemporaryRetain);

      expect(factory.mock.calls.length).toEqual(1);

      const [item, _disposePermanentRetain] =
        cacheItem.permanentRetainIfNotDisposed(mockedDisposeTemporaryRetain)!;

      expect(item).toEqual(1);
      expect(mockedDisposeTemporaryRetain.mock.calls.length).toEqual(1);

      const state = getState(cacheItem);

      assert(state.kind === 'NotInParentCacheAndNotDisposed');
      expect(state.disposeValue).toEqual(disposeItem);
      expect(state.permanentRetainCount).toEqual(1);
      expect(state.value).toEqual(1);
      expect(disposeItem.mock.calls.length).toEqual(0);
      expect(removeFromParentCache.mock.calls.length).toEqual(1);
    });

    test('Disposing the temporary retain after permanent retain throws', () => {
      const removeFromParentCache = vi.fn();
      const disposeItem = vi.fn();

      const factory = vi.fn(() => {
        const ret: ItemCleanupPair<number> = [1, disposeItem];
        return ret;
      });
      const [cacheItem, disposeTemporaryRetain] =
        createTemporarilyRetainedCacheItem<number>(
          factory,
          removeFromParentCache,
        );

      expect(factory.mock.calls.length).toEqual(1);

      const [item, _disposePermanentRetain] =
        cacheItem.permanentRetainIfNotDisposed(disposeTemporaryRetain)!;

      expect(() => {
        disposeTemporaryRetain();
      }).toThrow();
    });

    test('Item is disposed when the permanently retain is disposed', () => {
      const removeFromParentCache = vi.fn();
      const disposeItem = vi.fn();

      const factory = vi.fn(() => {
        const ret: ItemCleanupPair<number> = [1, disposeItem];
        return ret;
      });
      const [cacheItem, disposeTemporaryRetain] =
        createTemporarilyRetainedCacheItem<number>(
          factory,
          removeFromParentCache,
        );

      const mockedDisposeTemporaryRetain = vi.fn(disposeTemporaryRetain);

      expect(factory.mock.calls.length).toEqual(1);

      const [item, disposePermanentRetain] =
        cacheItem.permanentRetainIfNotDisposed(mockedDisposeTemporaryRetain)!;

      disposePermanentRetain();

      const state = getState(cacheItem);
      assert(state.kind === 'NotInParentCacheAndDisposed');
      expect(removeFromParentCache.mock.calls.length).toEqual(1);
      expect(disposeItem.mock.calls.length).toEqual(1);
    });

    test('Repeated disposal of permanently retained cache item throws', () => {
      const removeFromParentCache = vi.fn();
      const disposeItem = vi.fn();

      const factory = vi.fn(() => {
        const ret: ItemCleanupPair<number> = [1, disposeItem];
        return ret;
      });
      const [cacheItem, disposeTemporaryRetain] =
        createTemporarilyRetainedCacheItem<number>(
          factory,
          removeFromParentCache,
        );

      const mockedDisposeTemporaryRetain = vi.fn(disposeTemporaryRetain);

      expect(factory.mock.calls.length).toEqual(1);

      const [item, disposePermanentRetain] =
        cacheItem.permanentRetainIfNotDisposed(mockedDisposeTemporaryRetain)!;

      disposePermanentRetain();
      expect(() => {
        disposePermanentRetain();
      }).toThrow();
    });

    test('Permanent retain of cache item whose temporary retain has expired', () => {
      const removeFromParentCache = vi.fn();
      const disposeItem = vi.fn();

      const factory = vi.fn(() => {
        const ret: ItemCleanupPair<number> = [1, disposeItem];
        return ret;
      });
      const [cacheItem, disposeTemporaryRetain] =
        createTemporarilyRetainedCacheItem<number>(
          factory,
          removeFromParentCache,
        );

      expect(factory.mock.calls.length).toEqual(1);

      vi.advanceTimersToNextTimer();

      const state = getState(cacheItem);

      assert(state.kind === 'NotInParentCacheAndDisposed');

      assert(
        cacheItem.permanentRetainIfNotDisposed(disposeTemporaryRetain) === null,
      );

      expect(() => {
        cacheItem.permanentRetain();
      }).toThrow();
    });

    test('It is invalid to temporarily retain after the permanent retain', () => {
      const removeFromParentCache = vi.fn();
      const disposeItem = vi.fn();

      const factory = vi.fn(() => {
        const ret: ItemCleanupPair<number> = [1, disposeItem];
        return ret;
      });
      const [cacheItem, disposeTemporaryRetain1] =
        createTemporarilyRetainedCacheItem<number>(
          factory,
          removeFromParentCache,
        );

      const [_value, disposeOfPermanentRetain1] =
        cacheItem.permanentRetainIfNotDisposed(disposeTemporaryRetain1)!;

      expect(() => {
        cacheItem.temporaryRetain();
      }).toThrow();
    });
  });

  describe('Item that temporarily retained twice', () => {
    test('Cache item is not disposed after first temporary retain expires', () => {
      const removeFromParentCache = vi.fn();
      const disposeItem = vi.fn();

      const factory = vi.fn(() => {
        const ret: ItemCleanupPair<number> = [1, disposeItem];
        return ret;
      });
      const [cacheItem, _disposeTemporaryRetain1] =
        createTemporarilyRetainedCacheItem<number>(
          factory,
          removeFromParentCache,
        );

      vi.advanceTimersByTime(1000);

      const _disposeTemporaryRetain2 = cacheItem.temporaryRetain();

      vi.advanceTimersToNextTimer();

      const state = getState(cacheItem);
      assert(state.kind === 'InParentCacheAndNotDisposed');
      expect(state.value).toEqual(1);
      expect(state.permanentRetainCount).toEqual(0);
      expect(state.temporaryRetainCount).toEqual(1);
      expect(state.disposeValue).toEqual(disposeItem);
      expect(state.removeFromParentCache).toEqual(removeFromParentCache);
      expect(disposeItem.mock.calls.length).toEqual(0);
      expect(removeFromParentCache.mock.calls.length).toEqual(0);
    });

    test('Cache item is not disposed after first temporary retain is disposed', () => {
      const removeFromParentCache = vi.fn();
      const disposeItem = vi.fn();

      const factory = vi.fn(() => {
        const ret: ItemCleanupPair<number> = [1, disposeItem];
        return ret;
      });
      const [cacheItem, disposeTemporaryRetain1] =
        createTemporarilyRetainedCacheItem<number>(
          factory,
          removeFromParentCache,
        );

      vi.advanceTimersByTime(1000);
      const _disposeTemporaryRetain2 = cacheItem.temporaryRetain();

      disposeTemporaryRetain1();

      const state = getState(cacheItem);
      assert(state.kind === 'InParentCacheAndNotDisposed');
      expect(state.value).toEqual(1);
      expect(state.permanentRetainCount).toEqual(0);
      expect(state.temporaryRetainCount).toEqual(1);
      expect(state.disposeValue).toEqual(disposeItem);
      expect(state.removeFromParentCache).toEqual(removeFromParentCache);
      expect(disposeItem.mock.calls.length).toEqual(0);
      expect(removeFromParentCache.mock.calls.length).toEqual(0);
    });

    test('Disposing the first temporary retain after it expires is a no-op', () => {
      const removeFromParentCache = vi.fn();
      const disposeItem = vi.fn();

      const factory = vi.fn(() => {
        const ret: ItemCleanupPair<number> = [1, disposeItem];
        return ret;
      });
      const [cacheItem, disposeTemporaryRetain1] =
        createTemporarilyRetainedCacheItem<number>(
          factory,
          removeFromParentCache,
        );

      vi.advanceTimersByTime(1000);
      const _disposeTemporaryRetain2 = cacheItem.temporaryRetain();
      vi.advanceTimersToNextTimer();
      disposeTemporaryRetain1();

      const state = getState(cacheItem);
      assert(state.kind === 'InParentCacheAndNotDisposed');
      expect(state.temporaryRetainCount).toEqual(1);
    });

    test('Item is not disposed if only the second temporary retain is disposed', () => {
      const removeFromParentCache = vi.fn();
      const disposeItem = vi.fn();

      const factory = vi.fn(() => {
        const ret: ItemCleanupPair<number> = [1, disposeItem];
        return ret;
      });
      const [cacheItem, disposeTemporaryRetain1] =
        createTemporarilyRetainedCacheItem<number>(
          factory,
          removeFromParentCache,
        );

      vi.advanceTimersByTime(1000);
      const disposeTemporaryRetain2 = cacheItem.temporaryRetain();
      disposeTemporaryRetain2();

      const state = getState(cacheItem);
      assert(state.kind === 'InParentCacheAndNotDisposed');
      expect(state.temporaryRetainCount).toEqual(1);
    });

    test('Item is disposed if both temporary retains are disposed', () => {
      const removeFromParentCache = vi.fn();
      const disposeItem = vi.fn();

      const factory = vi.fn(() => {
        const ret: ItemCleanupPair<number> = [1, disposeItem];
        return ret;
      });
      const [cacheItem, disposeTemporaryRetain1] =
        createTemporarilyRetainedCacheItem<number>(
          factory,
          removeFromParentCache,
        );

      const disposeTemporaryRetain2 = cacheItem.temporaryRetain();
      const state1 = getState(cacheItem);
      assert(state1.kind === 'InParentCacheAndNotDisposed');
      expect(state1.temporaryRetainCount).toEqual(2);
      disposeTemporaryRetain1();
      expect(state1.temporaryRetainCount).toEqual(1);
      disposeTemporaryRetain2();

      const state2 = getState(cacheItem);
      assert(state2.kind === 'NotInParentCacheAndDisposed');
    });

    test('Item is disposed if both temporary retains are disposed in reverse order', () => {
      const removeFromParentCache = vi.fn();
      const disposeItem = vi.fn();

      const factory = vi.fn(() => {
        const ret: ItemCleanupPair<number> = [1, disposeItem];
        return ret;
      });
      const [cacheItem, disposeTemporaryRetain1] =
        createTemporarilyRetainedCacheItem<number>(
          factory,
          removeFromParentCache,
        );

      const disposeTemporaryRetain2 = cacheItem.temporaryRetain();
      const state1 = getState(cacheItem);
      assert(state1.kind === 'InParentCacheAndNotDisposed');
      expect(state1.temporaryRetainCount).toEqual(2);
      disposeTemporaryRetain2();
      expect(state1.temporaryRetainCount).toEqual(1);
      disposeTemporaryRetain1();

      const state2 = getState(cacheItem);
      assert(state2.kind === 'NotInParentCacheAndDisposed');
    });

    test('Item is disposed if both temporary retains expire', () => {
      const removeFromParentCache = vi.fn();
      const disposeItem = vi.fn();

      const factory = vi.fn(() => {
        const ret: ItemCleanupPair<number> = [1, disposeItem];
        return ret;
      });
      const [cacheItem, _disposeTemporaryRetain1] =
        createTemporarilyRetainedCacheItem<number>(
          factory,
          removeFromParentCache,
        );

      vi.advanceTimersByTime(1000);
      const _disposeTemporaryRetain2 = cacheItem.temporaryRetain();

      const state1 = getState(cacheItem);
      assert(state1.kind === 'InParentCacheAndNotDisposed');
      expect(state1.temporaryRetainCount).toEqual(2);

      vi.advanceTimersToNextTimer();
      expect(state1.temporaryRetainCount).toEqual(1);

      vi.advanceTimersToNextTimer();
      const state = getState(cacheItem);
      assert(state.kind === 'NotInParentCacheAndDisposed');
      expect(disposeItem.mock.calls.length).toEqual(1);
      expect(removeFromParentCache.mock.calls.length).toEqual(1);
    });
  });

  describe('Item that is temporarily retained twice and then permanently retained', () => {
    test('Item is not removed from the parent cache when the permanent retain is created', () => {
      const removeFromParentCache = vi.fn();
      const disposeItem = vi.fn();

      const factory = vi.fn(() => {
        const ret: ItemCleanupPair<number> = [1, disposeItem];
        return ret;
      });
      const [cacheItem, disposeTemporaryRetain1] =
        createTemporarilyRetainedCacheItem<number>(
          factory,
          removeFromParentCache,
        );

      vi.advanceTimersByTime(1000);
      const _disposeTemporaryRetain2 = cacheItem.temporaryRetain();

      const [_value, _disposeOfPermanentRetain] =
        cacheItem.permanentRetainIfNotDisposed(disposeTemporaryRetain1)!;

      const state = getState(cacheItem);

      assert(state.kind === 'InParentCacheAndNotDisposed');
      expect(state.value).toEqual(1);
      expect(state.permanentRetainCount).toEqual(1);
      expect(state.temporaryRetainCount).toEqual(1);
    });

    test('Item is removed from the parent cache when the second temporary retain is disposed', () => {
      const removeFromParentCache = vi.fn();
      const disposeItem = vi.fn();

      const factory = vi.fn(() => {
        const ret: ItemCleanupPair<number> = [1, disposeItem];
        return ret;
      });
      const [cacheItem, disposeTemporaryRetain1] =
        createTemporarilyRetainedCacheItem<number>(
          factory,
          removeFromParentCache,
        );

      vi.advanceTimersByTime(1000);
      const disposeTemporaryRetain2 = cacheItem.temporaryRetain();

      const [_value, _disposeOfPermanentRetain] =
        cacheItem.permanentRetainIfNotDisposed(disposeTemporaryRetain1)!;

      const state1 = getState(cacheItem);
      assert(state1.kind === 'InParentCacheAndNotDisposed');
      expect(state1.permanentRetainCount).toEqual(1);
      expect(state1.temporaryRetainCount).toEqual(1);

      disposeTemporaryRetain2();

      const state2 = getState(cacheItem);
      assert(state2.kind === 'NotInParentCacheAndNotDisposed');
      expect(state2.permanentRetainCount).toEqual(1);
    });

    test('Item is removed from the parent cache when the second temporary retain expires', () => {
      const removeFromParentCache = vi.fn();
      const disposeItem = vi.fn();

      const factory = vi.fn(() => {
        const ret: ItemCleanupPair<number> = [1, disposeItem];
        return ret;
      });
      const [cacheItem, disposeTemporaryRetain1] =
        createTemporarilyRetainedCacheItem<number>(
          factory,
          removeFromParentCache,
        );

      vi.advanceTimersByTime(1000);
      const _disposeTemporaryRetain2 = cacheItem.temporaryRetain();

      const [_value, _disposeOfPermanentRetain] =
        cacheItem.permanentRetainIfNotDisposed(disposeTemporaryRetain1)!;

      const state1 = getState(cacheItem);
      assert(state1.kind === 'InParentCacheAndNotDisposed');
      expect(state1.permanentRetainCount).toEqual(1);
      expect(state1.temporaryRetainCount).toEqual(1);

      vi.advanceTimersToNextTimer();

      const state2 = getState(cacheItem);
      assert(state2.kind === 'NotInParentCacheAndNotDisposed');
      expect(state2.permanentRetainCount).toEqual(1);
    });

    test('Item is not removed from the parent cache when the permanent retain is disposed', () => {
      const removeFromParentCache = vi.fn();
      const disposeItem = vi.fn();

      const factory = vi.fn(() => {
        const ret: ItemCleanupPair<number> = [1, disposeItem];
        return ret;
      });
      const [cacheItem, disposeTemporaryRetain1] =
        createTemporarilyRetainedCacheItem<number>(
          factory,
          removeFromParentCache,
        );

      vi.advanceTimersByTime(1000);
      const _disposeTemporaryRetain2 = cacheItem.temporaryRetain();

      const [_value, disposeOfPermanentRetain] =
        cacheItem.permanentRetainIfNotDisposed(disposeTemporaryRetain1)!;

      disposeOfPermanentRetain();

      const state = getState(cacheItem);
      assert(state.kind === 'InParentCacheAndNotDisposed');
      expect(state.permanentRetainCount).toEqual(0);
      expect(state.temporaryRetainCount).toEqual(1);
    });

    test('Item is disposed the permanent retain is disposed and the second temporary retain expires', () => {
      const removeFromParentCache = vi.fn();
      const disposeItem = vi.fn();

      const factory = vi.fn(() => {
        const ret: ItemCleanupPair<number> = [1, disposeItem];
        return ret;
      });
      const [cacheItem, disposeTemporaryRetain1] =
        createTemporarilyRetainedCacheItem<number>(
          factory,
          removeFromParentCache,
        );

      vi.advanceTimersByTime(1000);
      const _disposeTemporaryRetain2 = cacheItem.temporaryRetain();

      const [_value, disposeOfPermanentRetain] =
        cacheItem.permanentRetainIfNotDisposed(disposeTemporaryRetain1)!;

      disposeOfPermanentRetain();
      vi.advanceTimersToNextTimer();

      const state = getState(cacheItem);
      assert(state.kind === 'NotInParentCacheAndDisposed');
    });

    test('Item is disposed when the permanent retain is disposed and the second temporary retain is disposed', () => {
      const removeFromParentCache = vi.fn();
      const disposeItem = vi.fn();

      const factory = vi.fn(() => {
        const ret: ItemCleanupPair<number> = [1, disposeItem];
        return ret;
      });
      const [cacheItem, disposeTemporaryRetain1] =
        createTemporarilyRetainedCacheItem<number>(
          factory,
          removeFromParentCache,
        );

      vi.advanceTimersByTime(1000);
      const disposeTemporaryRetain2 = cacheItem.temporaryRetain();

      const [_value, disposeOfPermanentRetain] =
        cacheItem.permanentRetainIfNotDisposed(disposeTemporaryRetain1)!;

      disposeOfPermanentRetain();
      disposeTemporaryRetain2();

      const state = getState(cacheItem);
      assert(state.kind === 'NotInParentCacheAndDisposed');
    });

    test('Item is disposed when the second temporary is disposed and the permanent retain is disposed', () => {
      const removeFromParentCache = vi.fn();
      const disposeItem = vi.fn();

      const factory = vi.fn(() => {
        const ret: ItemCleanupPair<number> = [1, disposeItem];
        return ret;
      });
      const [cacheItem, disposeTemporaryRetain1] =
        createTemporarilyRetainedCacheItem<number>(
          factory,
          removeFromParentCache,
        );

      vi.advanceTimersByTime(1000);
      const disposeTemporaryRetain2 = cacheItem.temporaryRetain();

      const [_value, disposeOfPermanentRetain] =
        cacheItem.permanentRetainIfNotDisposed(disposeTemporaryRetain1)!;

      disposeTemporaryRetain2();
      disposeOfPermanentRetain();

      const state = getState(cacheItem);
      assert(state.kind === 'NotInParentCacheAndDisposed');
    });

    test('Item is disposed when the second temporary expires and the permanent retain is disposed', () => {
      const removeFromParentCache = vi.fn();
      const disposeItem = vi.fn();

      const factory = vi.fn(() => {
        const ret: ItemCleanupPair<number> = [1, disposeItem];
        return ret;
      });
      const [cacheItem, disposeTemporaryRetain1] =
        createTemporarilyRetainedCacheItem<number>(
          factory,
          removeFromParentCache,
        );

      vi.advanceTimersByTime(1000);
      const _disposeTemporaryRetain2 = cacheItem.temporaryRetain();

      const [_value, disposeOfPermanentRetain] =
        cacheItem.permanentRetainIfNotDisposed(disposeTemporaryRetain1)!;

      vi.advanceTimersToNextTimer();
      disposeOfPermanentRetain();

      const state = getState(cacheItem);
      assert(state.kind === 'NotInParentCacheAndDisposed');
    });
  });

  describe('Multiple permanent retains', () => {
    test('Item is only disposed when second permanent retain is disposed', () => {
      const removeFromParentCache = vi.fn();
      const disposeItem = vi.fn();

      const factory = vi.fn(() => {
        const ret: ItemCleanupPair<number> = [1, disposeItem];
        return ret;
      });
      const [cacheItem, disposeTemporaryRetain1] =
        createTemporarilyRetainedCacheItem<number>(
          factory,
          removeFromParentCache,
        );

      vi.advanceTimersByTime(1000);
      const disposeTemporaryRetain2 = cacheItem.temporaryRetain();

      const [_value, disposeOfPermanentRetain1] =
        cacheItem.permanentRetainIfNotDisposed(disposeTemporaryRetain1)!;
      const [_value2, disposeOfPermanentRetain2] =
        cacheItem.permanentRetainIfNotDisposed(disposeTemporaryRetain2)!;

      disposeOfPermanentRetain1();
      const state = getState(cacheItem);
      assert(state.kind === 'NotInParentCacheAndNotDisposed');
      expect(state.permanentRetainCount).toEqual(1);

      disposeOfPermanentRetain2();
      const state2 = getState(cacheItem);
      assert(state2.kind === 'NotInParentCacheAndDisposed');
    });
  });
});

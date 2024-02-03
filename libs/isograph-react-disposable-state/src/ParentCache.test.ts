import { describe, assert, test, vi, expect } from 'vitest';
import { ParentCache } from './ParentCache';
import { ItemCleanupPair } from '@isograph/disposable-types';
import { CacheItem } from './CacheItem';

function getValue<T>(cache: ParentCache<T>): CacheItem<T> | null {
  return (cache as any).__cacheItem as CacheItem<T> | null;
}

describe('ParentCache', () => {
  test(
    'Populated, emptied, repopulated cache is not ' +
      're-emptied by original temporary retain being disposed',
    () => {
      const factory = vi.fn(() => {
        const pair: ItemCleanupPair<number> = [1, vi.fn()];
        return pair;
      });
      const parentCache = new ParentCache<number>(factory);

      const [_cacheItem, value, clearTemporaryRetain] =
        parentCache.getOrPopulateAndTemporaryRetain();

      expect(factory.mock.calls.length).toBe(1);
      assert(value === 1);
      assert(getValue(parentCache) != null, 'Parent cache should not be empty');

      parentCache.empty();
      assert(getValue(parentCache) === null);

      parentCache.getOrPopulateAndTemporaryRetain();
      expect(factory.mock.calls.length).toBe(2);

      assert(getValue(parentCache) != null);

      clearTemporaryRetain();

      assert(getValue(parentCache) != null);
    },
  );

  test('Clearing the only temporary retain removes the item from the parent cache', () => {
    const factory = vi.fn(() => {
      const pair: ItemCleanupPair<number> = [1, vi.fn()];
      return pair;
    });
    const parentCache = new ParentCache<number>(factory);

    const [_cacheItem, _value, clearTemporaryRetain] =
      parentCache.getOrPopulateAndTemporaryRetain();
    clearTemporaryRetain();

    assert(getValue(parentCache) === null);
  });

  test('Clearing one of two temporary retains does not remove the item from the parent cache', () => {
    const factory = vi.fn(() => {
      const pair: ItemCleanupPair<number> = [1, vi.fn()];
      return pair;
    });
    const parentCache = new ParentCache<number>(factory);

    const [_cacheItem, _value, clearTemporaryRetain] =
      parentCache.getOrPopulateAndTemporaryRetain();
    const [_cacheItem2, _value2, clearTemporaryRetain2] =
      parentCache.getOrPopulateAndTemporaryRetain();

    clearTemporaryRetain();
    assert(getValue(parentCache) != null);

    clearTemporaryRetain2();
    assert(getValue(parentCache) === null);
  });
});

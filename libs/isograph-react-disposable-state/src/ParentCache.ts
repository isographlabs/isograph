import { CacheItem, createTemporarilyRetainedCacheItem } from './CacheItem';
import {
  CleanupFn,
  Factory,
  ItemCleanupPair,
} from '@isograph/disposable-types';

// TODO convert cache impl to a getter and setter and free functions
// TODO accept options that get passed to CacheItem

/**
 * ParentCache
 * - A ParentCache can be in two states: populated and unpopulated.
 * - A ParentCache holds a CacheItem, which can choose to remove itself from
 *   the parent ParentCache.
 * - If the ParentCache is populated, the CacheItem (i.e. this.__value) must be
 *   in the InParentCacheAndNotDisposed state, i.e. not disposed, so after we
 *   null-check this.__value, this.__value.getValue(), this.__value.temporaryRetain()
 *   and this.__value.permanentRetain() are safe to be called.
 *
 * - Though we do not do so, it is always safe to call parentCache.delete().
 *
 * Invariant:
 * - A parent cache at a given "location" (conceptually, an ID) should always
 *   be called
 */
export class ParentCache<T> {
  private __cacheItem: CacheItem<T> | null = null;
  private readonly __factory: Factory<T>;

  // TODO pass an onEmpty function, which can e.g. remove this ParentCache
  // from some parent object.
  constructor(factory: Factory<T>) {
    this.__factory = factory;
  }

  /**
   * This is called from useCachedResponsivePrecommitValue, when the parent cache is populated
   * and a previous temporary retain has been disposed. This can occur in scenarios like:
   * - temporary retain A is created by component B rendering
   * - temporary retain A expires, emptying the parent cache
   * - another component renders, sharing the same parent cache, filling
   *   by calling getOrPopulateAndTemporaryRetain
   * - component B commits. We see that temporary retain A has been disposed,
   *   and re-check the parent cache by calling this method.
   */
  getAndPermanentRetainIfPresent(): ItemCleanupPair<T> | null {
    return this.__cacheItem != null
      ? [this.__cacheItem.getValue(), this.__cacheItem.permanentRetain()]
      : null;
  }

  getOrPopulateAndTemporaryRetain(): [CacheItem<T>, T, CleanupFn] {
    return this.__cacheItem === null
      ? this.__populateAndTemporaryRetain()
      : temporaryRetain(this.__cacheItem);
  }

  private __populateAndTemporaryRetain(): [CacheItem<T>, T, CleanupFn] {
    const pair: ItemCleanupPair<CacheItem<T>> =
      createTemporarilyRetainedCacheItem(this.__factory, () => {
        // We are doing this check because we don't want to remove the cache item
        // if it is not the one that was created when the temporary retain was created.
        //
        // Consider the following scenario:
        // - we populate the cache with CacheItem A,
        // - then manually delete CacheItem A (e.g. to force a refetch)
        // - then, we re-populate the parent cache with CacheItem B
        // - then, the temporary retain of CacheItem A is disposed or expires.
        //
        // At this point, we don't want to delete CacheItem B from the cache.
        //
        // TODO consider what happens if items are === comparable to each other,
        // e.g. the item is a number!
        if (this.__cacheItem === pair[0]) {
          this.empty();
        }
      });

    // We deconstruct this here instead of at the definition site because otherwise,
    // typescript thinks that cacheItem is any, because it's referenced in the closure.
    const [cacheItem, disposeTemporaryRetain] = pair;
    this.__cacheItem = cacheItem;
    return [cacheItem, cacheItem.getValue(), disposeTemporaryRetain];
  }

  empty() {
    this.__cacheItem = null;
  }

  get factory(): Factory<T> {
    return this.__factory;
  }

  isEmpty(): boolean {
    return this.__cacheItem === null;
  }
}

function temporaryRetain<T>(value: CacheItem<T>): [CacheItem<T>, T, CleanupFn] {
  return [value, value.getValue(), value.temporaryRetain()];
}

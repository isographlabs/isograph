"use strict";

import { useEffect, useState } from "react";
import { ParentCache } from "./ParentCache";
import { useHasCommittedRef } from "./useHasCommittedRef";
import { ItemCleanupPair } from "@isograph/isograph-disposable-types/dist";

/**
 * usePrecommitValue<T>
 * - Takes a mutable parent cache, a factory function, and an onCommit callback.
 * - Returns T before the initial commit, and null afterward.
 * - Calls onCommit with the ItemCleanupPair during the first commit.
 * - The T from the render phase is only temporarily retained. It may have been
 *   disposed by the time of the commit. If so, this hook checks the parent cache
 *   for another T or creates one, and passes this T to onCommit.
 * - If the T returned during the last render is not the same as the one that
 *   is passed to onCommit, during the commit phase, will schedule another render.
 *
 * Invariant: the returned T has not been disposed during the tick of the render.
 * The T passed to the onCommit callback has not been disposed when the onCommit
 * callback is called.
 *
 * Passing a different parentCache:
 * - Pre-commit, passing a different parentCache has the effect of "resetting" this
 *   hook's state to the new cache's state. For example, if you have a cache associated
 *   with a set of variables (e.g. {name: "Matthew"}), and pass in another cache
 *   (e.g. associated with {name: "James"}), which is empty, the hook will fill that
 *   new cache with the factory function.
 *
 * Passing a different factory:
 * - Passing a different factory has no effect, except when factory is called,
 *   which is when the parent cache is being filled, or during the initial commit.
 *
 * Passing a different onCommit:
 * - Passing a different onCommit has no effect, except for during the initial commit.
 *
 * Post-commit, all parameters are ignored and the hook returns null.
 */
export function useCachedPrecommitValue<T>(
  parentCache: ParentCache<T>,
  onCommit: (pair: ItemCleanupPair<T>) => void
): { state: T } | null {
  // TODO: there should be two APIs. One in which we always re-render if the
  // committed item was not returned during the last render, and one in which
  // we do not. The latter is useful for cases where every disposable item
  // behaves identically, but must be loaded.
  //
  // This hook is the former, i.e. re-renders if the committed item has changed.
  const [, rerender] = useState<{} | null>(null);

  useEffect(() => {
    // On first commit, cacheItem may be disposed, because during the render phase,
    // we only temporarily retained the item, and the temporary retain could have
    // expired by the time of the commit.
    //
    // So, we can be in one of two states:
    // - the item is not disposed. In that case, permanently retain and use that item.
    // - the item is disposed. In that case, we can be in two states:
    //   - the parent cache is not empty (due to another component rendering, or
    //     another render of the same component.) In that case, permanently retain and
    //     use the item from the parent cache. (Note: any item present in the parent
    //     cache is not disposed.)
    //   - the parent cache is empty. In that case, call factory, getting a new item
    //     and a cleanup function.
    //
    // After the above, we have a non-disposed item and a cleanup function, which we
    // can pass to onCommit.
    const undisposedPair = cacheItem.permanentRetainIfNotDisposed(
      disposeOfTemporaryRetain
    );
    if (undisposedPair !== null) {
      onCommit(undisposedPair);
    } else {
      // The cache item we created during render has been disposed. Check if the parent
      // cache is populated.
      const existingCacheItemCleanupPair =
        parentCache.getAndPermanentRetainIfPresent();
      if (existingCacheItemCleanupPair !== null) {
        onCommit(existingCacheItemCleanupPair);
      } else {
        // We did not find an item in the parent cache, create a new one.
        onCommit(parentCache.factory());
      }

      // TODO: Consider whether we always want to rerender if the committed item
      // was not returned during the last render, or whether some callers will
      // prefer opting out of this behavior (e.g. if every disposable item behaves
      // identically, but must be loaded.)
      rerender({});
    }
  }, []);

  const hasCommittedRef = useHasCommittedRef();
  if (hasCommittedRef.current) {
    return null;
  }

  // Safety: item is only safe to use (i.e. guaranteed not to have disposed)
  // during this tick.
  const [cacheItem, item, disposeOfTemporaryRetain] =
    parentCache.getOrPopulateAndTemporaryRetain();

  return { state: item };
}

'use strict';

import { useEffect, useState } from 'react';

import { ParentCache } from './ParentCache';
import { useCachedPrecommitValue } from './useCachedPrecommitValue';
import {
  UNASSIGNED_STATE,
  useUpdatableDisposableState,
  type UnassignedState,
} from './useUpdatableDisposableState';

/**
 * useLazyDisposableState<T>
 * - Takes a mutable parent cache and a factory function
 * - Returns { state: T }
 *
 * This lazily loads the disposable item using useCachedPrecommitValue, then
 * (on commit) sets it in state. The item continues to be returned after
 * commit and is disposed when the hook unmounts.
 */
export function useLazyDisposableState<T>(
  parentCache: ParentCache<Exclude<T, UnassignedState>>,
): {
  state: T;
} {
  const { state: item, setState: setItemCleanupPair } =
    useUpdatableDisposableState<T>();

  function refetch(parentCache: ParentCache<Exclude<T, UnassignedState>>) {
    const undisposedPair = parentCache.getAndPermanentRetainIfPresent();

    if (undisposedPair !== null) {
      setItemCleanupPair(undisposedPair);
    } else {
      setItemCleanupPair(parentCache.factory());
    }
  }

  const preCommitItem = useCachedPrecommitValue(parentCache, (pair) => {
    setItemCleanupPair(pair);
  });

  const [initialCache] = useState(parentCache);

  useEffect(() => {
    if (initialCache === parentCache) return;

    refetch(parentCache);
  }, [parentCache]);

  const returnedItem =
    preCommitItem?.state ?? (item !== UNASSIGNED_STATE ? item : null);

  if (returnedItem != null) {
    // return refetch?
    return { state: returnedItem };
  }

  // Safety: This can't happen. For renders before the initial commit, preCommitItem
  // is non-null. During the initial commit, we assign itemCleanupPairRef.current,
  // so during subsequent renders, itemCleanupPairRef.current is non-null.
  throw new Error(
    'returnedItem was unexpectedly null. This indicates a bug in react-disposable-state.',
  );
}

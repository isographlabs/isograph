'use strict';

import type { ItemCleanupPair } from '@isograph/disposable-types';
import { useEffect, useRef } from 'react';
import type { ParentCache } from './ParentCache';
import { useCachedResponsivePrecommitValue } from './useCachedResponsivePrecommitValue';
import { type UnassignedState } from './useUpdatableDisposableState';

/**
 * useLazyDisposableState<T>
 * - Takes a mutable parent cache and a factory function
 * - Returns { state: T }
 *
 * This lazily loads the disposable item using useCachedResponsivePrecommitValue, then
 * (on commit) sets it in state. The item continues to be returned after
 * commit and is disposed when the hook unmounts.
 */
export function useLazyDisposableState<T>(
  parentCache: ParentCache<Exclude<T, UnassignedState>>,
): {
  state: T;
} {
  const itemCleanupPairRef = useRef<ItemCleanupPair<T> | null>(null);
  const preCommitItem = useCachedResponsivePrecommitValue(
    parentCache,
    (pair) => {
      itemCleanupPairRef.current = pair;
    },
  );

  const lastCommittedParentCache = useRef<ParentCache<T> | null>(null);
  useEffect(() => {
    if (lastCommittedParentCache.current === parentCache) {
      return;
    }
    lastCommittedParentCache.current = parentCache;

    return () => {
      const cleanupFn = itemCleanupPairRef.current?.[1];
      // TODO confirm useEffect is called in order.
      if (cleanupFn == null) {
        throw new Error(
          'cleanupFn unexpectedly null. This indicates a bug in react-disposable-state.',
        );
      }
      return cleanupFn();
    };
  }, [parentCache]);

  const returnedItem = preCommitItem?.state ?? itemCleanupPairRef.current?.[0];

  if (returnedItem != null) {
    return { state: returnedItem };
  }

  // Safety: This can't happen. For renders before the initial commit, preCommitItem
  // is non-null. During the initial commit, we assign itemCleanupPairRef.current,
  // so during subsequent renders, itemCleanupPairRef.current is non-null.
  throw new Error(
    'returnedItem was unexpectedly null. This indicates a bug in react-disposable-state.',
  );
}

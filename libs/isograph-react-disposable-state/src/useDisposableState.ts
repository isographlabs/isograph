import { ItemCleanupPair } from '@isograph/disposable-types';
import { useEffect, useRef } from 'react';
import { ParentCache } from './ParentCache';

import { useCachedResponsivePrecommitValue } from './useCachedResponsivePrecommitValue';
import {
  UNASSIGNED_STATE,
  UnassignedState,
  useUpdatableDisposableState,
} from './useUpdatableDisposableState';

type UseUpdatableDisposableStateReturnValue<T> = {
  state: T;
  setState: (pair: ItemCleanupPair<Exclude<T, UnassignedState>>) => void;
};

export function useDisposableState<T = never>(
  parentCache: ParentCache<T>,
): UseUpdatableDisposableStateReturnValue<T> {
  const itemCleanupPairRef = useRef<ItemCleanupPair<T> | null>(null);

  const preCommitItem = useCachedResponsivePrecommitValue(
    parentCache,
    (pair) => {
      itemCleanupPairRef.current?.[1]();
      itemCleanupPairRef.current = pair;
    },
  );

  const { state: stateFromDisposableStateHook, setState } =
    useUpdatableDisposableState<T>();

  useEffect(
    function cleanupItemCleanupPairRefAfterSetState() {
      if (stateFromDisposableStateHook !== UNASSIGNED_STATE) {
        if (itemCleanupPairRef.current !== null) {
          itemCleanupPairRef.current[1]();
          itemCleanupPairRef.current = null;
        } else {
          throw new Error(
            'itemCleanupPairRef.current is unexpectedly null. ' +
              'This indicates a bug in react-disposable-state.',
          );
        }
      }
    },
    [stateFromDisposableStateHook],
  );

  useEffect(function cleanupItemCleanupPairRefIfSetStateNotCalled() {
    return () => {
      if (itemCleanupPairRef.current !== null) {
        itemCleanupPairRef.current[1]();
        itemCleanupPairRef.current = null;
      }
    };
  }, []);

  // Safety: we can be in one of three states. Pre-commit, in which case
  // preCommitItem is assigned, post-commit but before setState has been
  // called, in which case itemCleanupPairRef.current is assigned, or
  // after setState has been called, in which case
  // stateFromDisposableStateHook is assigned.
  //
  // Therefore, the type of state is T, not T | undefined. But the fact
  // that we are in one of the three states is not reflected in the types.
  // So we have to cast to T.
  //
  // Note that in the post-commit post-setState state, itemCleanupPairRef
  // can still be assigned, during the render before the
  // cleanupItemCleanupPairRefAfterSetState effect is called.
  const state: T | undefined =
    (stateFromDisposableStateHook != UNASSIGNED_STATE
      ? stateFromDisposableStateHook
      : null) ??
    preCommitItem?.state ??
    itemCleanupPairRef.current?.[0];

  return {
    state: state!,
    setState,
  };
}

// @ts-ignore
function tsTests() {
  let x: any;
  const a = useDisposableState(x);
  // This should be a compiler error, because the generic is inferred to be of
  // type never. TODO determine why this doesn't break the build!
  // @ts-expect-error
  a.setState(['asdf', () => {}]);
  // @ts-expect-error
  a.setState([UNASSIGNED_STATE, () => {}]);
  const b = useDisposableState<string | UnassignedState>(x);
  // @ts-expect-error
  b.setState([UNASSIGNED_STATE, () => {}]);
  b.setState(['asdf', () => {}]);
}

import type { Factory, ItemCleanupPair } from '@isograph/disposable-types';
import { useCallback, useInsertionEffect, useRef, useState } from 'react';
import { useHasCommittedRef } from './useHasCommittedRef';
import {
  UNASSIGNED_STATE,
  type UnassignedState,
} from './useUpdatableDisposableState';

export type UseDisposableStateOnceReturnValue<T> = {
  state: T | UnassignedState;
  setStateOnce: (factory: Factory<Exclude<T, UnassignedState>>) => void;
};

// useDisposableStateOnce
// - Returns a { state, setStateOnce } object. state is UNASSIGNED_STATE until the first setStateOnce call puts an item
//   in state.
// - setStateOnce takes a factory rather than an ItemCleanupPair. The first call calls the factory and puts the item in
//   state. Every later call does nothing and does not call its factory, so several event handlers can each call
//   setStateOnce and only one item is created.
// - setStateOnce throws if called before the initial commit, as useUpdatableDisposableState's setState does. A call
//   after unmount creates nothing.
// - The item is disposed when the component unmounts, and only then. The cleanup is an insertion effect's, which React
//   runs when it deletes the component, but not when an Activity hides it or on StrictMode's simulated unmount. A
//   component hidden by an Activity therefore keeps its item, and shows the same item when it is shown again.
// - React 18 and React 19 before 19.2 skip insertion effect cleanups when they delete a subtree that a Suspense
//   boundary or an Activity is hiding (fixed by https://github.com/facebook/react/pull/34372). On those versions, an
//   item whose component is deleted while hidden is never disposed.
// - Because the item is disposed inside an insertion effect, its cleanup must not update React state synchronously.
//   React logs "useInsertionEffect must not schedule updates" if it does.
export function useDisposableStateOnce<
  T = never,
>(): UseDisposableStateOnceReturnValue<T> {
  const [state, setState] = useState<T | UnassignedState>(UNASSIGNED_STATE);
  const hasCommittedRef = useHasCommittedRef();
  const hasBeenCalledRef = useRef(false);
  const isDeletedRef = useRef(false);
  const pairRef = useRef<ItemCleanupPair<Exclude<T, UnassignedState>> | null>(
    null,
  );

  const setStateOnce = useCallback(
    (factory: Factory<Exclude<T, UnassignedState>>) => {
      if (!hasCommittedRef.current) {
        throw new Error(
          'Calling setStateOnce before the component commits is disallowed.',
        );
      }
      if (hasBeenCalledRef.current) {
        return;
      }
      hasBeenCalledRef.current = true;
      if (isDeletedRef.current) {
        return;
      }
      const pair = factory();
      pairRef.current = pair;
      // The updater form, because T may be a function.
      setState(() => pair[0]);
    },
    [hasCommittedRef],
  );

  useInsertionEffect(
    () => () => {
      isDeletedRef.current = true;
      const pair = pairRef.current;
      pairRef.current = null;
      pair?.[1]();
    },
    [],
  );

  return { state, setStateOnce };
}

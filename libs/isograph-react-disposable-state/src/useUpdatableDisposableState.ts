import { ItemCleanupPair } from '@isograph/disposable-types';
import { useCallback, useEffect, useRef, useState } from 'react';
import { useHasCommittedRef } from './useHasCommittedRef';

export const UNASSIGNED_STATE: unique symbol = Symbol();
export type UnassignedState = typeof UNASSIGNED_STATE;

type UseUpdatableDisposableStateReturnValue<T> = {
  state: T | UnassignedState;
  setState: (pair: ItemCleanupPair<Exclude<T, UnassignedState>>) => void;
};

/**
 * ICI stands for ItemCleanupIndex. Use a short name because it shows up a lot.
 *
 * Like an ItemCleanupPair<T>, but with an additional index and as a struct
 * for readability.
 *
 * Interally, we must keep track of the index of the state change. The disposable
 * items that are set in state are not guaranteed to be distinct according to ===,
 * as in `setState([1, dispose1]); setState([1, dispose2])`. Following this, we
 * would expect dispose1 to be called on commit. If state items were compared for
 * ===, then dispose1 would not be called in such a situation.
 *
 * Note that this comes at a runtime cost, and we may want to expose APIs that do
 * not incur this runtime cost, for example in cases where every disposable item is
 * distinguishable via ===.
 */
type ICI<T> = { item: T; cleanup: () => void; index: number };

/**
 * useUpdatableDisposableState
 * - Returns a { state, setItem } object.
 * - setItem accepts an ItemCleanupPair<T>, and throws if called before commit.
 * - setItem sets the T in state and adds it to a set.
 * - React's behavior is that when the hook commits, whatever item is currently
 *   returned from the useState hook is the oldest item which will ever be returned
 *   from that useState hook. (More newly created ones can later be returned with
 *   concurrent mode.)
 * - When this hook commits, all items up to, but not including, the item currently
 *   returned from the useState hook are disposed and removed from the set.
 *
 * Calling setState before the hook commits:
 * - Calling setState before the hook commits is disallowed because until the hook
 *   commits, React will not schedule any unmount callbacks, meaning that if this
 *   hook never commits, any disposable items passed to setState will never be
 *   disposed.
 * - We also cannot store them in some cache, because multiple components can share
 *   the same cache location (for example, if they are loading the same query from
 *   multiple components), so updating the cache with the disposable item will cause
 *   both components to show the updated data, which is almost certainly a bug.
 * - Note that calling setState before commit is probably an anti-pattern! Consider
 *   not doing it.
 * - If you must, the workaround is to lazily load the disposable item with
 *   useDisposableState or useLazyDisposableState, and update the cache location
 *   instead of calling setState before commit. One can update the cache location
 *   by calling setState on some parent component that has already mounted, and
 *   therefore passing in different props.
 *   - This may only work in concurrent mode, though.
 */
export function useUpdatableDisposableState<
  T = never,
>(): UseUpdatableDisposableStateReturnValue<T> {
  const hasCommittedRef = useHasCommittedRef();

  const undisposedICIs = useRef(new Set<ICI<T>>());
  const setStateCountRef = useRef(0);

  const [stateICI, setStateICI] = useState<ICI<T> | UnassignedState>(
    UNASSIGNED_STATE,
  );

  const setStateAfterCommit = useCallback(
    (itemCleanupPair: ItemCleanupPair<T>) => {
      if (!hasCommittedRef.current) {
        throw new Error(
          'Calling setState before the component has committed is unsafe and disallowed.',
        );
      }

      const ici: ICI<T> = {
        item: itemCleanupPair[0],
        cleanup: itemCleanupPair[1],
        index: setStateCountRef.current,
      };
      setStateCountRef.current++;
      undisposedICIs.current.add(ici);
      setStateICI(ici);
    },
    [setStateICI],
  );

  useEffect(function cleanupUnreachableItems() {
    const indexInState = stateICI !== UNASSIGNED_STATE ? stateICI.index : 0;

    if (indexInState === 0) {
      return;
    }

    for (const undisposedICI of undisposedICIs.current) {
      if (undisposedICI.index === indexInState) {
        break;
      }
      undisposedICIs.current.delete(undisposedICI);
      undisposedICI.cleanup();
    }
  });

  useEffect(() => {
    return function disposeAllRemainingItems() {
      for (const undisposedICI of undisposedICIs.current) {
        undisposedICI.cleanup();
      }
    };
  }, []);

  return {
    setState: setStateAfterCommit,
    state: stateICI !== UNASSIGNED_STATE ? stateICI.item : UNASSIGNED_STATE,
  };
}

// @ts-ignore
function tsTests() {
  const a = useUpdatableDisposableState();
  // @ts-expect-error
  a.setState([UNASSIGNED_STATE, () => {}]);
  // @ts-expect-error
  a.setState(['asdf', () => {}]);
  const b = useUpdatableDisposableState<string | UnassignedState>();
  // @ts-expect-error
  b.setState([UNASSIGNED_STATE, () => {}]);
  b.setState(['asdf', () => {}]);
}

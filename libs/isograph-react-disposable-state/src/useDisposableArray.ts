import type { ItemCleanupPair } from '@isograph/disposable-types';
import { useCallback } from 'react';
import {
  UNASSIGNED_STATE,
  useUpdatableDisposableState,
} from './useUpdatableDisposableState';

export type UseDisposableArrayReturn<T> = {
  // The array this render shows. It is empty until the first setEntries call commits. The hook owns every pair in
  // it, so callers read the items and never call the cleanups.
  readonly entries: ReadonlyArray<ItemCleanupPair<T>>;
  // Replaces the array. See useDisposableArray.
  readonly setEntries: (nextEntries: ReadonlyArray<ItemCleanupPair<T>>) => void;
};

const NO_ENTRIES: ReadonlyArray<never> = Object.freeze([]);

// useDisposableArray
// - Holds an array of disposable items in state, on top of useUpdatableDisposableState.
// - setEntries(nextEntries) stores nextEntries. Releasing a stored array calls the cleanup of each of its pairs once.
//   useUpdatableDisposableState releases every array that is older than the committed one after the commit, and the
//   rest on unmount, including arrays that never committed.
// - Every pair in nextEntries was acquired by the caller for this call, and the hook owns it from then on. To keep an
//   item of entries, the caller takes a new reference to it, for example with cloneIfNotDisposed() on a
//   ReferenceCountedPointer, and passes that new pair. The caller never passes a pair of entries itself and never
//   calls a cleanup the hook owns.
// - The caller builds nextEntries from the entries of its render. If that render is stale, because its entries were
//   released after a newer array committed, taking a new reference fails (cloneIfNotDisposed() returns null). The
//   caller then releases what it acquired and does not call setEntries.
// - Two setEntries calls before a commit both build on the same entries, and the second replaces the first.
// - Like useUpdatableDisposableState's setState, setEntries throws if called before the initial commit. It throws
//   before storing anything, so the caller still owns nextEntries and must release them.
export function useDisposableArray<T>(): UseDisposableArrayReturn<T> {
  const { state, setState } =
    useUpdatableDisposableState<ReadonlyArray<ItemCleanupPair<T>>>();

  const setEntries = useCallback(
    (nextEntries: ReadonlyArray<ItemCleanupPair<T>>) => {
      setState([
        nextEntries,
        () => {
          for (const entry of nextEntries) {
            entry[1]();
          }
        },
      ]);
    },
    [setState],
  );

  return {
    entries: state === UNASSIGNED_STATE ? NO_ENTRIES : state,
    setEntries,
  };
}

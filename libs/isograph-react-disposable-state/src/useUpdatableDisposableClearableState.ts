import type { ItemCleanupPair } from '@isograph/disposable-types';
import { useCallback } from 'react';
import {
  UNASSIGNED_STATE,
  type UnassignedState,
  useUpdatableDisposableState,
} from './useUpdatableDisposableState';

type UseUpdatableDisposableClearableStateReturnValue<T> = {
  state: T | UnassignedState;
  setState: (pair: ItemCleanupPair<Exclude<T, UnassignedState>>) => void;
  clearState: () => void;
};

/**
 * The item that `clearState` puts in the underlying state. It is never returned
 * from this hook; the hook reports UNASSIGNED_STATE while it is current.
 */
const CLEARED_STATE: unique symbol = Symbol();
type ClearedState = typeof CLEARED_STATE;

function noop() {}

/**
 * useUpdatableDisposableClearableState
 * - useUpdatableDisposableState, plus clearState.
 * - clearState returns the state to UNASSIGNED_STATE. The item that was in state
 *   is disposed on the next commit, exactly as if it had been superseded by
 *   setState.
 * - Like setState, clearState throws if called before the initial commit.
 *
 * This is a wrapper around useUpdatableDisposableState: clearing sets a sentinel
 * item with no cleanup, so the underlying hook's commit-time disposal of
 * superseded items disposes the cleared item, and the sentinel is mapped back to
 * UNASSIGNED_STATE on the way out.
 */
export function useUpdatableDisposableClearableState<
  T = never,
>(): UseUpdatableDisposableClearableStateReturnValue<T> {
  const { state, setState } = useUpdatableDisposableState<T | ClearedState>();

  const clearState = useCallback(() => {
    setState([CLEARED_STATE, noop]);
  }, [setState]);

  return {
    clearState,
    setState,
    state: state === CLEARED_STATE ? UNASSIGNED_STATE : state,
  };
}

// @ts-ignore
function tsTests() {
  const a = useUpdatableDisposableClearableState();
  // @ts-expect-error
  a.setState([UNASSIGNED_STATE, () => {}]);
  // @ts-expect-error
  a.setState(['asdf', () => {}]);
  const b = useUpdatableDisposableClearableState<string | UnassignedState>();
  // @ts-expect-error
  b.setState([UNASSIGNED_STATE, () => {}]);
  b.setState(['asdf', () => {}]);
  // The cleared sentinel is not part of T, so it cannot be set from outside.
  // @ts-expect-error
  b.setState([CLEARED_STATE, () => {}]);
}

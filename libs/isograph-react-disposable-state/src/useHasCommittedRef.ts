import { type MutableRefObject, useInsertionEffect, useRef } from 'react';

/**
 * Returns true if the component has committed, false otherwise.
 *
 * Set in an insertion effect, which React runs in the mutation phase of the commit, before any layout or passive
 * effect of the commit. Layout and passive effects of the component's children, and DOM event handlers that run
 * before the passive effects are flushed, therefore read true.
 */
export function useHasCommittedRef(): MutableRefObject<boolean> {
  const hasCommittedRef = useRef(false);
  useInsertionEffect(() => {
    hasCommittedRef.current = true;
  }, []);
  return hasCommittedRef;
}

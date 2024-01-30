import { MutableRefObject, useEffect, useRef } from 'react';

/**
 * Returns true if the component has committed, false otherwise.
 */
export function useHasCommittedRef(): MutableRefObject<boolean> {
  const hasCommittedRef = useRef(false);
  useEffect(() => {
    hasCommittedRef.current = true;
  }, []);
  return hasCommittedRef;
}

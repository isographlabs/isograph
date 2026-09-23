import { type MutableRefObject, useEffect, useRef } from 'react';

/**
 * Detects that React is running this component's effects again without a
 * render in between. That happens on StrictMode's simulated unmount and
 * remount on the first commit, on Fast Refresh, and when an `Activity`
 * boundary hides and then shows the component. In each case React runs every
 * effect's cleanup and then its mount, so a cleanup that sets a flag is
 * observable from the mounts that follow.
 *
 * Returns a ref whose `current` is `true` from the moment the cleanup ran until
 * the caller resets it. The effect that must tell a re-run apart from a first
 * mount reads it and sets it back to `false`; nothing else clears it.
 */
export function useEffectsRerunWithoutRenderRef(): MutableRefObject<boolean> {
  const effectsRerunWithoutRenderRef = useRef(false);
  useEffect(() => {
    return () => {
      effectsRerunWithoutRenderRef.current = true;
    };
  }, []);
  return effectsRerunWithoutRenderRef;
}

import { Ref, useEffect, useRef } from 'react';

export function useOnScreen(
  ref: React.RefObject<HTMLElement>,
  onVisible: (() => void) | null,
) {
  const isVisible = useRef(false);
  const observerRef: React.MutableRefObject<IntersectionObserver | null> =
    useRef<IntersectionObserver>(null);

  useEffect(() => {
    observerRef.current = new IntersectionObserver(([entry]) => {
      if (onVisible != null && entry.isIntersecting !== isVisible.current) {
        if (entry.isIntersecting) {
          onVisible?.();
        }
        isVisible.current = entry.isIntersecting;
      }
    });
    observerRef.current?.observe(ref.current!);
    return () => {
      observerRef.current?.disconnect();
      observerRef.current = null;
    };
  }, [ref, onVisible]);
}

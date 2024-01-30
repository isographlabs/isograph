/**
 * Invariant:
 * Before the promise is resolved, value becomes non-null.
 */
export type PromiseWrapper<T extends object> = {
  promise: Promise<T>;
  value: T | null;
};

export function wrapPromise<T extends object>(
  promise: Promise<T>,
): PromiseWrapper<T> {
  // TODO confirm suspense works if the promise is already resolved.
  const wrapper: PromiseWrapper<T> = { promise, value: null };
  promise.then((v) => {
    wrapper.value = v;
  });
  return wrapper;
}

export function useReadPromise<T extends object>(p: PromiseWrapper<T>): T {
  if (p.value != null) {
    return p.value;
  }
  throw p.promise;
}

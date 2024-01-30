const NOT_SET: Symbol = Symbol('NOT_SET');
type NotSet = typeof NOT_SET;

/**
 * Invariant:
 * Before the promise is resolved, value becomes non-null.
 */
export type PromiseWrapper<T> = {
  promise: Promise<T>;
  value: Exclude<T, NotSet> | NotSet;
};

export function wrapPromise<T>(promise: Promise<T>): PromiseWrapper<T> {
  // TODO confirm suspense works if the promise is already resolved.
  const wrapper: PromiseWrapper<T> = { promise, value: NOT_SET };
  promise.then((v) => {
    // T is assignable to Exclude<T, Symbol> | Symbol
    wrapper.value = v as any;
  });
  return wrapper;
}

export function useReadPromise<T>(p: PromiseWrapper<T>): T {
  if (p.value !== NOT_SET) {
    // Safety: p.value is either NOT_SET or an actual value.
    return p.value as any;
  }
  throw p.promise;
}

import type { CleanupFn, ItemCleanupPair } from '@isograph/disposable-types';

export const NOT_SET = Symbol('NOT_SET');
export type NotSet = typeof NOT_SET;

export type Result<T, E> =
  | {
      readonly kind: 'Ok';
      readonly value: T;
    }
  | {
      readonly kind: 'Err';
      readonly error: E;
    };

/**
 * Invariant:
 * Before the promise is resolved, value becomes non-null.
 */
export type PromiseWrapper<T, E = unknown> = {
  readonly promise: Promise<Exclude<T, NotSet>>;
  result: Result<Exclude<T, NotSet>, E> | NotSet;
};

export interface PromiseWrapperOk<T, E = unknown> extends PromiseWrapper<T, E> {
  result: {
    readonly kind: 'Ok';
    readonly value: Exclude<T, NotSet>;
  };
}

export function wrapPromise<T>(
  promise: Promise<Exclude<T, NotSet>>,
): PromiseWrapper<T, unknown> {
  const wrapper: PromiseWrapper<T, unknown> = { promise, result: NOT_SET };
  promise
    .then((v) => {
      wrapper.result = { kind: 'Ok', value: v };
    })
    .catch((e: unknown) => {
      wrapper.result = { kind: 'Err', error: e };
    });
  return wrapper;
}

export function wrapResolvedValue<T>(
  value: Exclude<T, NotSet>,
): PromiseWrapperOk<T, never> {
  return {
    promise: Promise.resolve(value),
    result: {
      kind: 'Ok',
      value,
    },
  };
}

export function readPromise<T, E>(p: PromiseWrapper<T, E>): T {
  const { result } = p;
  if (result !== NOT_SET) {
    const resultKind = result;
    if (resultKind.kind === 'Ok') {
      return resultKind.value;
    } else {
      throw resultKind.error;
    }
  }

  throw p.promise;
}

export type PromiseState<T, E> =
  | {
      readonly kind: 'Pending';
      readonly promise: Promise<T>;
    }
  | Result<T, E>;

export function getPromiseState<T, E>(
  p: PromiseWrapper<T, E>,
): PromiseState<T, E> {
  if (p.result !== NOT_SET) {
    return p.result;
  }
  return {
    kind: 'Pending',
    promise: p.promise,
  };
}

/** Where a `transpose` cleanup stands. */
type TransposeStatus =
  /** The cleanup has not run, and the inner pair has not arrived. */
  | { readonly kind: 'Waiting' }
  /** The inner pair arrived, and the cleanup has not run. */
  | { readonly kind: 'Arrived'; readonly cleanup: CleanupFn }
  /** The cleanup ran before the inner pair arrived, or after `wrapper` rejected; the inner
   * cleanup runs if the pair arrives. */
  | { readonly kind: 'ClearedBeforeArrival' }
  /** The cleanup ran after the inner pair arrived. */
  | { readonly kind: 'Cleared' };

/**
 * A disposable item that arrives later, as an item whose value arrives later. The returned
 * wrapper settles as `wrapper` does, with the inner item or the same error; the returned
 * cleanup runs the inner cleanup once the inner pair exists.
 */
export function transpose<T>(
  wrapper: PromiseWrapper<ItemCleanupPair<Exclude<T, NotSet>>, unknown>,
): ItemCleanupPair<PromiseWrapper<T, unknown>> {
  let status: TransposeStatus =
    wrapper.result !== NOT_SET && wrapper.result.kind === 'Ok'
      ? { kind: 'Arrived', cleanup: wrapper.result.value[1] }
      : { kind: 'Waiting' };
  const transposed = wrapPromise<T>(
    wrapper.promise.then(([item, cleanup]) => {
      switch (status.kind) {
        case 'Waiting':
          status = { kind: 'Arrived', cleanup };
          break;
        case 'ClearedBeforeArrival':
          status = { kind: 'Cleared' };
          cleanup();
          break;
        case 'Arrived':
        case 'Cleared':
          break;
      }
      return item;
    }),
  );
  if (wrapper.result !== NOT_SET) {
    transposed.result =
      wrapper.result.kind === 'Ok'
        ? { kind: 'Ok', value: wrapper.result.value[0] }
        : { kind: 'Err', error: wrapper.result.error };
  }
  return [
    transposed,
    () => {
      switch (status.kind) {
        case 'Waiting':
          status = { kind: 'ClearedBeforeArrival' };
          return;
        case 'Arrived': {
          const cleanup = status.cleanup;
          status = { kind: 'Cleared' };
          cleanup();
          return;
        }
        case 'ClearedBeforeArrival':
        case 'Cleared':
          throw new Error('A transposed cleanup should only be called once.');
      }
    },
  ];
}

export type AnyError = any;

export const NOT_SET = Symbol('NOT_SET');
export type NotSet = typeof NOT_SET;

export type Result<T, E> =
  | {
      kind: 'Ok';
      value: T;
    }
  | {
      kind: 'Err';
      error: E;
    };

/**
 * Invariant:
 * Before the promise is resolved, value becomes non-null.
 */
export type PromiseWrapper<T, E = any> = {
  readonly promise: Promise<Exclude<T, NotSet>>;
  result: Result<Exclude<T, NotSet>, E> | NotSet;
};

export function wrapPromise<T>(
  promise: Promise<Exclude<T, NotSet>>,
): PromiseWrapper<T, unknown> {
  // TODO confirm suspense works if the promise is already resolved.
  const wrapper: PromiseWrapper<T, any> = { promise, result: NOT_SET };
  promise
    .then((v) => {
      wrapper.result = { kind: 'Ok', value: v };
    })
    .catch((e) => {
      wrapper.result = { kind: 'Err', error: e };
    });
  return wrapper;
}

export function wrapResolvedValue<T>(
  value: Exclude<T, NotSet>,
): PromiseWrapper<T, never> {
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
      kind: 'Pending';
      promise: Promise<T>;
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

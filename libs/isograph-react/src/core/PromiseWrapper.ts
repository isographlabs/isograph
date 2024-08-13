export type AnyError = any;

const NOT_SET: Symbol = Symbol('NOT_SET');
type NotSet = typeof NOT_SET;

type Result<T, E> =
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
  readonly promise: Promise<T>;
  result: Result<Exclude<T, NotSet>, E> | NotSet;
};

export function wrapPromise<T>(promise: Promise<T>): PromiseWrapper<T, any> {
  // TODO confirm suspense works if the promise is already resolved.
  const wrapper: PromiseWrapper<T, any> = { promise, result: NOT_SET };
  promise
    .then((v) => {
      // v is assignable to Exclude<T, Symbol> | Symbol
      wrapper.result = { kind: 'Ok', value: v as any };
    })
    .catch((e) => {
      // e is assignable to Exclude<T, Symbol> | Symbol
      wrapper.result = { kind: 'Err', error: e as any };
    });
  return wrapper;
}

export function wrapResolvedValue<T>(value: T): PromiseWrapper<T, never> {
  return {
    promise: Promise.resolve(value),
    result: {
      kind: 'Ok',
      // @ts-expect-error one should not call wrapResolvedValue with NOT_SET
      value,
    },
  };
}

export function readPromise<T, E>(p: PromiseWrapper<T, E>): T {
  const { result } = p;
  if (result !== NOT_SET) {
    // Safety: p.result is either NOT_SET or an actual value.
    const resultKind = result as Result<T, any>;
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
  const { result } = p;
  if (result !== NOT_SET) {
    // Safety: p.result is either NOT_SET or an actual value.
    const resultKind = result as Result<T, any>;
    return resultKind;
  }
  return {
    kind: 'Pending',
    promise: p.promise,
  };
}

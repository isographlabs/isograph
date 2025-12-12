export type AnyError = any;

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
export type PromiseWrapper<T, E = any> = {
  readonly promise: Promise<T>;
  result: Result<T, E> | NotSet;
};

export interface PromiseWrapperOk<T, E = any> extends PromiseWrapper<T, E> {
  result: {
    readonly kind: 'Ok';
    readonly value: T;
  };
}

export interface PromiseWrapperErr<T, E = any> extends PromiseWrapper<T, E> {
  result: {
    readonly kind: 'Err';
    readonly error: E;
  };
}

export function wrapPromise<T>(
  promise: Promise<T>,
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

export function wrapResolvedValue<T>(value: T): PromiseWrapperOk<T, never> {
  return {
    promise: Promise.resolve(value),
    result: {
      kind: 'Ok',
      value,
    },
  };
}

export function wrapRejectedValue<E>(error: E): PromiseWrapperErr<never, E> {
  const promise = Promise.reject(error);
  promise.catch(() => {});
  return {
    promise: promise,
    result: {
      kind: 'Err',
      error,
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

function isPromiseWrapper(
  value: unknown,
): value is PromiseWrapper<unknown, unknown> {
  return (
    value != null &&
    typeof value === 'object' &&
    'promise' in value &&
    'result' in value
  );
}

type Unwrap<T> = T extends null | undefined
  ? T
  : T extends PromiseWrapper<infer V>
    ? V
    : T;

export function PromiseWrapperAll<T extends readonly unknown[] | []>(
  values: T,
): PromiseWrapper<
  {
    -readonly [P in keyof T]: Unwrap<T[P]>;
  },
  Unwrap<T[number]>
> {
  const promise: Promise<any> = Promise.all(
    values.map((value) => {
      if (!isPromiseWrapper(value)) {
        return value;
      }
      return value.promise;
    }),
  );

  const results = [];

  for (const value of values) {
    if (!isPromiseWrapper(value)) {
      results.push(value);
      continue;
    }

    const state = getPromiseState(value);
    switch (state.kind) {
      case 'Err': {
        promise.catch(() => {});
        return {
          promise: promise,
          result: {
            kind: 'Err',
            error: state.error as any,
          },
        };
      }
      case 'Pending':
        return wrapPromise(promise) as any;
      case 'Ok': {
        results.push(state.value);
      }
    }
  }

  promise.catch(() => {});
  return {
    promise: promise,
    result: {
      kind: 'Ok',
      value: results as any,
    },
  };
}

export function PromiseWrapperThen<T, E, TResult1 = T, TResult2 = never>(
  promiseWrapper: PromiseWrapper<T, E>,
  onfulfilled: (value: T) => TResult1,
  onrejected: (reason: E) => TResult2,
): PromiseWrapper<TResult1 | TResult2, never> {
  const state = getPromiseState(promiseWrapper);

  switch (state.kind) {
    case 'Pending':
      return wrapPromise(
        state.promise.then(onfulfilled, onrejected),
      ) satisfies PromiseWrapper<
        TResult1 | TResult2,
        unknown
      > as PromiseWrapper<TResult1 | TResult2, never>;
    case 'Err': {
      const result = onrejected(state.error);

      return {
        promise: Promise.resolve(result),
        result: {
          kind: 'Ok',
          value: result,
        },
      };
    }
    case 'Ok': {
      const result = onfulfilled(state.value);

      return {
        promise: Promise.resolve(result),
        result: {
          kind: 'Ok',
          value: result,
        },
      };
    }
  }
}

export type AnyError = any;

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
  result:
    | Result<T, E>
    | {
        readonly kind: 'Pending';
      };
};

export interface PromiseWrapperOk<T> extends PromiseWrapper<T, never> {
  result: {
    readonly kind: 'Ok';
    readonly value: T;
  };
}

export interface PromiseWrapperErr<E> extends PromiseWrapper<never, E> {
  result: {
    readonly kind: 'Err';
    readonly error: E;
  };
}

export function wrapPromise<T>(
  promise: Promise<T>,
): PromiseWrapper<T, unknown> {
  // TODO confirm suspense works if the promise is already resolved.
  const wrapper: PromiseWrapper<T, any> = {
    promise,
    result: {
      kind: 'Pending',
    },
  };

  promise
    .then((v) => {
      wrapper.result = { kind: 'Ok', value: v };
    })
    .catch((e) => {
      wrapper.result = { kind: 'Err', error: e };
    });
  return wrapper;
}

export function wrapResolvedValue<T>(value: T): PromiseWrapperOk<T> {
  return {
    promise: Promise.resolve(value),
    result: {
      kind: 'Ok',
      value,
    },
  };
}

export function wrapRejectedValue<E>(error: E): PromiseWrapperErr<E> {
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

  if (result.kind === 'Ok') {
    return result.value;
  } else if (result.kind === 'Err') {
    throw result.error;
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
  if (p.result.kind !== 'Pending') {
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

type UnwrapErr<T> = T extends null | undefined
  ? never
  : T extends PromiseWrapper<infer _, infer E>
    ? E
    : never;

export function all<T extends readonly unknown[] | []>(
  values: T,
): PromiseWrapper<
  {
    -readonly [P in keyof T]: Unwrap<T[P]>;
  },
  UnwrapErr<T[keyof T]>
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

export function mapErr<T, E, TResult>(
  promiseWrapper: PromiseWrapper<T, E>,
  onrejected: (reason: E) => TResult,
): PromiseWrapper<T, TResult> {
  const state = getPromiseState(promiseWrapper);

  switch (state.kind) {
    case 'Pending':
      return wrapPromise(
        state.promise.then(null, (e) => Promise.reject(onrejected(e))),
      ) satisfies PromiseWrapper<T, unknown> as PromiseWrapper<T, TResult>;
    case 'Err': {
      const result = onrejected(state.error);

      const promise = Promise.reject(result);
      promise.catch(() => {});
      return {
        promise: promise,
        result: {
          kind: 'Err',
          error: result,
        },
      };
    }
    case 'Ok': {
      return {
        promise: promiseWrapper.promise,
        result: {
          kind: 'Ok',
          value: state.value,
        },
      };
    }
  }
}

export function mapOk<T, E, TResult>(
  promiseWrapper: PromiseWrapper<T, E>,
  onfulfilled: (value: T) => TResult,
): PromiseWrapper<TResult, E> {
  const state = getPromiseState(promiseWrapper);

  switch (state.kind) {
    case 'Pending':
      return wrapPromise(
        state.promise.then(onfulfilled),
      ) satisfies PromiseWrapper<TResult, unknown> as PromiseWrapper<
        TResult,
        E
      >;
    case 'Err': {
      return {
        promise: promiseWrapper.promise as unknown as Promise<TResult>,
        result: {
          kind: 'Err',
          error: state.error,
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

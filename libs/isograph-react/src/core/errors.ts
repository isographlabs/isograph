import type { Brand } from './brand';
import type { NetworkResponseError, NetworkResponseErrorPath } from './cache';
import type { StoreError, WithErrors } from './IsographEnvironment';
import { isNonEmptyArray, type NonEmptyArray } from './NonEmptyArray';
import type { ReadFieldErrors } from './read';

declare const NetworkResponseErrorPathJoinedBrand: unique symbol;
type NetworkResponseErrorPathJoined = Brand<
  string,
  typeof NetworkResponseErrorPathJoinedBrand
>;

export class ReadFieldAggregateError extends AggregateError {
  readonly errors!: ReadFieldError[];
  constructor(errors: NonEmptyArray<ReadFieldErrors>, message?: string) {
    super(
      errors.flatMap(({ errors, path }) =>
        errors.map((error) => new ReadFieldError(error, path)),
      ),
      message,
    );
    this.name = new.target.name;
  }
}

export type ReadFieldErrorPath = string | number;
export class ReadFieldError extends Error {
  constructor(
    readonly error: StoreError,
    readonly path: ReadFieldErrorPath[],
  ) {
    super('Read field error');
    this.name = new.target.name;
  }
}

function joinNetworkResponseErrorPath(
  path: readonly NetworkResponseErrorPath[] | undefined,
): NetworkResponseErrorPathJoined {
  return (path?.join('.') ?? '') as NetworkResponseErrorPathJoined;
}

export type ErrorsByPath = Map<
  NetworkResponseErrorPathJoined,
  NonEmptyArray<NetworkResponseError>
>;

export function groupErrorsByPath(
  errors: readonly NetworkResponseError[],
): ErrorsByPath {
  return groupBy(errors, (error) => joinNetworkResponseErrorPath(error.path));
}

/**
 * If errors bubble up, the error path will be a full-path to the field
 */
export function findErrors(
  errorsByPath: ErrorsByPath,
  path: readonly NetworkResponseErrorPath[],
) {
  const joinedPath = joinNetworkResponseErrorPath(path);
  let errors: StoreError[] = [];
  for (const [errorPath, suberrors] of errorsByPath) {
    if (suberrors != null && errorPath.startsWith(joinedPath)) {
      errors.push(
        ...suberrors.map(({ extensions, locations }) => ({
          extensions,
          locations,
        })),
      );
    }
  }
  return isNonEmptyArray(errors) ? errors : undefined;
}

export function readDataWithErrors<T>(
  result: WithErrors<T, ReadFieldErrors>,
  errors: ReadFieldErrors[],
): T | null {
  switch (result.kind) {
    case 'Errors':
      errors.push(...result.errors);
      return null;
    case 'Data': {
      return result.value;
    }
  }
}

function groupBy<V, K extends string | number | symbol>(
  arr: readonly V[],
  keyFn: (v: V) => K,
) {
  const result: Map<K, [V, ...V[]]> = new Map();
  for (const el of arr) {
    const key = keyFn(el);
    const entry = result.get(key);
    if (entry != null) {
      entry.push(el);
    } else {
      result.set(key, [el]);
    }
  }
  return result;
}

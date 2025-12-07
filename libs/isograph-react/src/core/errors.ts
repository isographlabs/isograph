import type { Brand } from './brand';
import type { NonEmptyArray } from './NonEmptyArray';

export interface PayloadErrorExtensions {}
export type PayloadErrorPath = string | number;
export type PayloadError = {
  readonly message: string;
  readonly locations?: { readonly line: number; readonly column: number }[];
  readonly path?: PayloadErrorPath[];
  readonly extensions?: PayloadErrorExtensions;
};

declare const PayloadErrorPathJoinedBrand: unique symbol;
type PayloadErrorPathJoined = Brand<string, typeof PayloadErrorPathJoinedBrand>;

export class GraphqlAggregateError extends AggregateError {
  readonly errors!: GraphqlError[];
  constructor(errors: Iterable<GraphqlError>, message?: string) {
    super(errors, message);
    this.name = 'GraphqlAggregateError';
  }
}

export class GraphqlError extends Error implements PayloadError {
  readonly locations?: { line: number; column: number }[];
  readonly path?: (string | number)[];
  readonly extensions?: PayloadErrorExtensions;
  constructor(error: PayloadError) {
    super(error.message);
    this.name = 'GraphqlError';
    if (error.path) this.path = error.path;
    if (error.locations) this.locations = error.locations;
    if (error.extensions) this.extensions = error.extensions;
  }
}

function joinPayloadErrorPath(
  path: PayloadErrorPath[] | undefined,
): PayloadErrorPathJoined {
  return (path?.join('.') ?? '') as PayloadErrorPathJoined;
}

export type ErrorsByPath = Map<
  PayloadErrorPathJoined,
  NonEmptyArray<PayloadError>
>;

export function groupErrorsByPath(errors: PayloadError[]): ErrorsByPath {
  return groupBy(errors, (error) => joinPayloadErrorPath(error.path));
}

/**
 * If errors bubble up, the error path will be a full-path to the field
 */
export function findErrors(
  errorsByPath: ErrorsByPath,
  path: PayloadErrorPath[],
) {
  const joinedPath = joinPayloadErrorPath(path);
  let errors: NonEmptyArray<PayloadError> | undefined = undefined;
  for (const [errorPath, suberrors] of errorsByPath) {
    if (suberrors && errorPath.startsWith(joinedPath)) {
      if (errors == null) {
        errors = suberrors;
      } else {
        errors.push(...suberrors);
      }
    }
  }
  return errors;
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

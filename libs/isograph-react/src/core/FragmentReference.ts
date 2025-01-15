import { type Link } from './IsographEnvironment';
import { ReaderWithRefetchQueries } from '../core/entrypoint';
import { PromiseWrapper } from './PromiseWrapper';
import type { StartUpdate } from './reader';

// TODO type this better
export type VariableValue = string | number | boolean | null | object;

export type Variables = { readonly [index: string]: VariableValue };

export type ExtractData<T> = T extends {
  data: infer D extends object;
}
  ? D
  : never;

export type ExtractParameters<T> = T extends {
  parameters: infer P extends Variables;
}
  ? P
  : Variables;

export type ExtractStartUpdate<
  T extends {
    startUpdate?: StartUpdate<object>;
  },
> = T['startUpdate'];

export type FragmentReference<
  TReadFromStore extends {
    parameters: object;
    data: object;
    startUpdate?: StartUpdate<object>;
  },
  TClientFieldValue,
> = {
  readonly kind: 'FragmentReference';
  readonly readerWithRefetchQueries: PromiseWrapper<
    ReaderWithRefetchQueries<TReadFromStore, TClientFieldValue>
  >;
  readonly root: Link;
  readonly variables: ExtractParameters<TReadFromStore>;
  readonly networkRequest: PromiseWrapper<void, any>;
};

export function stableIdForFragmentReference(
  fragmentReference: FragmentReference<any, any>,
): string {
  return `${fragmentReference.root.__typename}/${fragmentReference.root.__link}/TODO_FRAGMENT_NAME/${serializeVariables(fragmentReference.variables ?? {})}`;
}

function serializeVariables(variables: Variables) {
  let s = '';
  const keys = Object.keys(variables);
  keys.sort();
  for (const key of keys) {
    s += `${key}:${variables[key]},`;
  }
  return s;
}

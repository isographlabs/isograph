import { ReaderWithRefetchQueries } from '../core/entrypoint';
import { stableCopy } from './cache';
import { type Link } from './IsographEnvironment';
import { PromiseWrapper } from './PromiseWrapper';
import type { StartUpdate } from './reader';

// TODO type this better
export type VariableValue = string | number | boolean | null | object;

export type Variables = { readonly [index: string]: VariableValue };

export type UnknownTReadFromStore = {
  parameters: object;
  data: object;
  startUpdate?: StartUpdate<object>;
};

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
  TReadFromStore extends UnknownTReadFromStore,
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

export type StableIdForFragmentReference = string;

export function stableIdForFragmentReference(
  fragmentReference: FragmentReference<any, any>,
): StableIdForFragmentReference {
  return `${fragmentReference.root.__typename}/${fragmentReference.root.__link}/TODO_FRAGMENT_NAME/${JSON.stringify(stableCopy(fragmentReference.variables))}`;
}

import { ReaderWithRefetchQueries } from '../core/entrypoint';
import { stableCopy } from './cache';
import { type StoreLink } from './IsographEnvironment';
import { PromiseWrapper } from './PromiseWrapper';
import type { StartUpdate } from './reader';

// TODO type this better
export type VariableValue =
  | string
  | number
  | boolean
  | null
  | {
      readonly [index: string]: VariableValue;
    }
  | VariableValue[];

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

export type ExtractStartUpdate<T extends UnknownTReadFromStore> =
  T['startUpdate'];

export type ExtractUpdatableData<T extends UnknownTReadFromStore> =
  ExtractUpdatableDataFromStartUpdate<ExtractStartUpdate<T>>;

export type ExtractUpdatableDataFromStartUpdate<T> =
  T extends StartUpdate<infer D> ? D : never;

export type FragmentReference<
  TReadFromStore extends UnknownTReadFromStore,
  TClientFieldValue,
> = {
  readonly kind: 'FragmentReference';
  readonly readerWithRefetchQueries: PromiseWrapper<
    ReaderWithRefetchQueries<TReadFromStore, TClientFieldValue>
  >;
  readonly root: StoreLink;
  // TODO we potentially stably copy and stringify variables a lot!
  // So, we should employ interior mutability: pretend that fragent reference
  // is immutable, but actually store something like
  // `Map<Variable, StablyCopiedStringifiedOutput>`
  // and read or update that map when we would otherwise stably copy and
  // stringify.
  readonly variables: ExtractParameters<TReadFromStore>;
  readonly networkRequest: PromiseWrapper<void, any>;
};

export type StableIdForFragmentReference = string;

export function stableIdForFragmentReference(
  fragmentReference: FragmentReference<any, any>,
  fieldName: string,
): StableIdForFragmentReference {
  return `${fragmentReference.root.__typename}/${fragmentReference.root.__link}/${fieldName}/${JSON.stringify(stableCopy(fragmentReference.variables))}`;
}

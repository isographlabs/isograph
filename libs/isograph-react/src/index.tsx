import { DataTypeValue, Link } from './IsographEnvironment';
import { IsographEntrypoint } from './entrypoint';

export {
  retainQuery,
  unretainQuery,
  type RetainedQuery,
  garbageCollectEnvironment,
} from './garbageCollection';
export { type PromiseWrapper } from './PromiseWrapper';
export { makeNetworkRequest, subscribe } from './cache';
export {
  ROOT_ID,
  type DataId,
  type DataTypeValue,
  type IsographEnvironment,
  type IsographNetworkFunction,
  type IsographStore,
  type Link,
  type StoreRecord,
  createIsographEnvironment,
  createIsographStore,
  defaultMissingFieldHandler,
} from './IsographEnvironment';
export {
  IsographEnvironmentProvider,
  useIsographEnvironment,
  type IsographEnvironmentProviderProps,
} from './IsographEnvironmentProvider';
export { useImperativeReference } from './useImperativeReference';
export { EntrypointReader } from './EntrypointReader';
export {
  type ReaderArtifact,
  ReaderAst,
  ReaderAstNode,
  ReaderLinkedField,
  ReaderMutationField,
  ReaderRefetchField,
  ReaderResolverField,
  ReaderResolverVariant,
  ReaderScalarField,
} from './reader';
export {
  NormalizationAst,
  NormalizationAstNode,
  NormalizationLinkedField,
  NormalizationScalarField,
  IsographEntrypoint,
  assertIsEntrypoint,
  RefetchQueryArtifact,
  RefetchQueryArtifactWrapper,
} from './entrypoint';
export { read, readButDoNotEvaluate } from './read';
export { useResult } from './useResult';
export { type FragmentReference } from './FragmentReference';
export { useLazyReference } from './useLazyReference';

export type ExtractSecondParam<T extends (arg1: any, arg2: any) => any> =
  T extends (arg1: any, arg2: infer P) => any ? P : never;

export type Arguments = Argument[];
export type Argument = [ArgumentName, ArgumentValue];
export type ArgumentName = string;
export type ArgumentValue =
  | {
      kind: 'Variable';
      name: string;
    }
  | {
      kind: 'Literal';
      value: any;
    };

export type ExtractReadFromStore<Type> =
  Type extends IsographEntrypoint<infer X, any> ? X : never;
export type ExtractResolverResult<Type> =
  Type extends IsographEntrypoint<any, infer X> ? X : never;

export function assertLink(link: DataTypeValue): Link | null {
  if (Array.isArray(link)) {
    throw new Error('Unexpected array');
  }
  if (link == null) {
    return null;
  }
  if (typeof link === 'object') {
    return link;
  }
  throw new Error('Invalid link');
}

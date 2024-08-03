export {
  retainQuery,
  unretainQuery,
  type RetainedQuery,
  garbageCollectEnvironment,
} from './core/garbageCollection';
export { type PromiseWrapper } from './core/PromiseWrapper';
export { makeNetworkRequest, subscribe, normalizeData } from './core/cache';
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
} from './core/IsographEnvironment';
export {
  EagerReaderArtifact,
  ComponentReaderArtifact,
  RefetchReaderArtifact,
  ReaderAst,
  ReaderAstNode,
  ReaderLinkedField,
  ReaderNonLoadableResolverField,
  ReaderScalarField,
  TopLevelReaderArtifact,
  LoadableField,
} from './core/reader';
export {
  NormalizationAst,
  NormalizationAstNode,
  NormalizationLinkedField,
  NormalizationScalarField,
  IsographEntrypoint,
  assertIsEntrypoint,
  RefetchQueryNormalizationArtifact,
  RefetchQueryNormalizationArtifactWrapper,
} from './core/entrypoint';
export { readButDoNotEvaluate } from './core/read';
export {
  ExtractSecondParam,
  Argument,
  ArgumentName,
  ArgumentValue,
  Arguments,
} from './core/util';
export { type FragmentReference } from './core/FragmentReference';

export {
  IsographEnvironmentProvider,
  useIsographEnvironment,
  type IsographEnvironmentProviderProps,
} from './react/IsographEnvironmentProvider';
export { useImperativeReference } from './react/useImperativeReference';
export { EntrypointReader } from './react/EntrypointReader';
export { useResult } from './react/useResult';
export { useLazyReference } from './react/useLazyReference';
export { useRerenderOnChange } from './react/useRerenderOnChange';

export { useClientSideDefer } from './loadable-hooks/useClientSideDefer';

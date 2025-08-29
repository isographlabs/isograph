import type { UnknownTReadFromStore } from './FragmentReference';
import type { TypeName } from './IsographEnvironment';
import { TopLevelReaderArtifact } from './reader';
import { Arguments } from './util';

export type ReaderWithRefetchQueries<
  TReadFromStore extends UnknownTReadFromStore,
  TClientFieldValue,
> = {
  readonly kind: 'ReaderWithRefetchQueries';
  readonly readerArtifact: TopLevelReaderArtifact<
    TReadFromStore,
    TClientFieldValue,
    // TODO don't type this as any
    any
  >;
  readonly nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[];
};

export type ReaderWithRefetchQueriesLoader<
  TReadFromStore extends UnknownTReadFromStore,
  TClientFieldValue,
> = {
  readonly kind: 'ReaderWithRefetchQueriesLoader';
  readonly loader: () => Promise<
    ReaderWithRefetchQueries<TReadFromStore, TClientFieldValue>
  >;
};

export type NetworkRequestInfo<TNormalizationAst> = {
  readonly kind: 'NetworkRequestInfo';
  readonly operation: IsographOperation | IsographPersistedOperation;
  readonly normalizationAst: TNormalizationAst;
};

export type IsographOperation = {
  readonly kind: 'Operation';
  readonly text: string;
};

export type IsographPersistedOperation = {
  readonly kind: 'PersistedOperation';
  readonly operationId: string;
  readonly extraInfo: IsographPersistedOperationExtraInfo | null;
};

export type IsographPersistedOperationExtraInfo = {
  readonly kind: 'PersistedOperationExtraInfo';
  readonly operationName: string | null;
  readonly operationKind: 'Query' | 'Mutation' | 'Subscription';
};

// This type should be treated as an opaque type.
export type IsographEntrypoint<
  TReadFromStore extends UnknownTReadFromStore,
  TClientFieldValue,
  TNormalizationAst extends NormalizationAst | NormalizationAstLoader,
> = {
  readonly kind: 'Entrypoint';
  readonly networkRequestInfo: NetworkRequestInfo<TNormalizationAst>;
  readonly readerWithRefetchQueries:
    | ReaderWithRefetchQueries<TReadFromStore, TClientFieldValue>
    | ReaderWithRefetchQueriesLoader<TReadFromStore, TClientFieldValue>;
  readonly concreteType: TypeName;
};

export type IsographEntrypointLoader<
  TReadFromStore extends UnknownTReadFromStore,
  TClientFieldValue,
> = {
  readonly kind: 'EntrypointLoader';
  readonly typeAndField: string;
  readonly loader: () => Promise<
    IsographEntrypoint<TReadFromStore, TClientFieldValue, NormalizationAst>
  >;
};

export type NormalizationAstNode =
  | NormalizationScalarField
  | NormalizationLinkedField
  | NormalizationInlineFragment;

export type NormalizationAstNodes = ReadonlyArray<NormalizationAstNode>;

export type NormalizationAst = {
  readonly kind: 'NormalizationAst';
  readonly selections: NormalizationAstNodes;
};

export type NormalizationAstLoader = {
  readonly kind: 'NormalizationAstLoader';
  readonly loader: () => Promise<NormalizationAst>;
};

export type NormalizationScalarField = {
  readonly kind: 'Scalar';
  readonly fieldName: string;
  readonly arguments: Arguments | null;
};

export type NormalizationLinkedField = {
  readonly kind: 'Linked';
  readonly fieldName: string;
  readonly arguments: Arguments | null;
  readonly selections: NormalizationAstNodes;
  readonly concreteType: TypeName | null;
};

export type NormalizationInlineFragment = {
  readonly kind: 'InlineFragment';
  readonly type: string;
  readonly selections: NormalizationAstNodes;
};

// This is more like an entrypoint, but one specifically for a refetch query/mutation
export type RefetchQueryNormalizationArtifact = {
  readonly kind: 'RefetchQuery';
  readonly networkRequestInfo: NetworkRequestInfo<NormalizationAst>;
  readonly concreteType: TypeName;
};

// TODO rename
export type RefetchQueryNormalizationArtifactWrapper = {
  readonly artifact: RefetchQueryNormalizationArtifact;
  readonly allowedVariables: string[];
};

export function assertIsEntrypoint<
  TReadFromStore extends UnknownTReadFromStore,
  TClientFieldValue,
  TNormalizationAst extends NormalizationAst | NormalizationAstLoader,
>(
  value:
    | IsographEntrypoint<TReadFromStore, TClientFieldValue, TNormalizationAst>
    | ((_: any) => any)
    // Temporarily, allow any here. Once we automatically provide
    // types to entrypoints, we probably don't need this.
    | any,
): asserts value is IsographEntrypoint<
  TReadFromStore,
  TClientFieldValue,
  TNormalizationAst
> {
  if (typeof value === 'function') throw new Error('Not a string');
}

export type ExtractReadFromStore<Type> =
  Type extends IsographEntrypoint<infer X, any, any> ? X : never;
export type ExtractResolverResult<Type> =
  Type extends IsographEntrypoint<any, infer X, any> ? X : never;
export type ExtractProps<Type> = Type extends React.FC<infer X> ? X : never;

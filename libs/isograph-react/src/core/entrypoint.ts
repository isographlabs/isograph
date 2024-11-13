import type { TypeName } from './IsographEnvironment';
import { TopLevelReaderArtifact } from './reader';
import { Arguments } from './util';

export type ReaderWithRefetchQueries<
  TReadFromStore extends { parameters: object; data: object },
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

export type NetworkRequestInfo = {
  readonly kind: 'NetworkRequestInfo';
  readonly queryText: string;
  readonly normalizationAst: NormalizationAst;
};
// This type should be treated as an opaque type.
export type IsographEntrypoint<
  TReadFromStore extends { parameters: object; data: object },
  TClientFieldValue,
> = {
  readonly kind: 'Entrypoint';
  readonly networkRequestInfo: NetworkRequestInfo;
  readonly readerWithRefetchQueries: ReaderWithRefetchQueries<
    TReadFromStore,
    TClientFieldValue
  >;
  readonly concreteType: TypeName;
};

export type IsographEntrypointLoader<
  TReadFromStore extends { parameters: object; data: object },
  TClientFieldValue,
> = {
  readonly kind: 'EntrypointLoader';
  readonly typeAndField: string;
  readonly loader: () => Promise<
    IsographEntrypoint<TReadFromStore, TClientFieldValue>
  >;
};

export type NormalizationAstNode =
  | NormalizationScalarField
  | NormalizationLinkedField
  | NormalizationInlineFragment;
export type NormalizationAst = ReadonlyArray<NormalizationAstNode>;

export type NormalizationScalarField = {
  readonly kind: 'Scalar';
  readonly fieldName: string;
  readonly arguments: Arguments | null;
};

export type NormalizationLinkedField = {
  readonly kind: 'Linked';
  readonly fieldName: string;
  readonly arguments: Arguments | null;
  readonly selections: NormalizationAst;
  readonly concreteType: TypeName | null;
};

export type NormalizationInlineFragment = {
  readonly kind: 'InlineFragment';
  readonly type: string;
  readonly selections: NormalizationAst;
};

// This is more like an entrypoint, but one specifically for a refetch query/mutation
export type RefetchQueryNormalizationArtifact = {
  readonly kind: 'RefetchQuery';
  readonly networkRequestInfo: NetworkRequestInfo;
  readonly concreteType: TypeName;
};

// TODO rename
export type RefetchQueryNormalizationArtifactWrapper = {
  readonly artifact: RefetchQueryNormalizationArtifact;
  readonly allowedVariables: string[];
};

export function assertIsEntrypoint<
  TReadFromStore extends { parameters: object; data: object },
  TClientFieldValue,
>(
  value:
    | IsographEntrypoint<TReadFromStore, TClientFieldValue>
    | ((_: any) => any)
    // Temporarily, allow any here. Once we automatically provide
    // types to entrypoints, we probably don't need this.
    | any,
): asserts value is IsographEntrypoint<TReadFromStore, TClientFieldValue> {
  if (typeof value === 'function') throw new Error('Not a string');
}

export type ExtractReadFromStore<Type> =
  Type extends IsographEntrypoint<infer X, any> ? X : never;
export type ExtractResolverResult<Type> =
  Type extends IsographEntrypoint<any, infer X> ? X : never;
export type ExtractProps<Type> = Type extends React.FC<infer X> ? X : never;

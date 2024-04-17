import { ReaderArtifact } from './reader';
import { Arguments } from './util';

// This type should be treated as an opaque type.
export type IsographEntrypoint<
  TReadFromStore extends Object,
  TClientFieldValue,
> = {
  kind: 'Entrypoint';
  queryText: string;
  normalizationAst: NormalizationAst;
  readerArtifact: ReaderArtifact<TReadFromStore, TClientFieldValue>;
  nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[];
};

export type NormalizationAstNode =
  | NormalizationScalarField
  | NormalizationLinkedField;
export type NormalizationAst = NormalizationAstNode[];

export type NormalizationScalarField = {
  kind: 'Scalar';
  fieldName: string;
  arguments: Arguments | null;
};

export type NormalizationLinkedField = {
  kind: 'Linked';
  fieldName: string;
  arguments: Arguments | null;
  selections: NormalizationAst;
};

// This is more like an entrypoint, but one specifically for a refetch query/mutation
export type RefetchQueryNormalizationArtifact = {
  kind: 'RefetchQuery';
  queryText: string;
  normalizationAst: NormalizationAst;
};

// TODO rename
export type RefetchQueryNormalizationArtifactWrapper = {
  artifact: RefetchQueryNormalizationArtifact;
  allowedVariables: string[];
};

export function assertIsEntrypoint<
  TReadFromStore extends Object,
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

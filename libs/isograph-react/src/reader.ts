import { ComponentOrFieldName } from './IsographEnvironment';
import { Arguments } from './util';

// TODO this should probably be at least three distinct types, for @component,
// non-@component and refetch resolvers
export type ReaderArtifact<TReadFromStore extends Object, TClientFieldValue> = {
  readonly kind: 'ReaderArtifact';
  readonly fieldName: ComponentOrFieldName;
  readonly readerAst: ReaderAst<TReadFromStore>;
  // TODO move resolver into the variant
  readonly resolver: (
    data: TReadFromStore,
    runtimeProps: any,
  ) => TClientFieldValue;
  readonly variant: ReaderResolverVariant;
};

export type ReaderAstNode =
  | ReaderScalarField
  | ReaderLinkedField
  | ReaderResolverField
  | ReaderRefetchField
  | ReaderMutationField;

// @ts-ignore
export type ReaderAst<TReadFromStore> = ReadonlyArray<ReaderAstNode>;

export type ReaderScalarField = {
  readonly kind: 'Scalar';
  readonly fieldName: string;
  readonly alias: string | null;
  readonly arguments: Arguments | null;
};
export type ReaderLinkedField = {
  readonly kind: 'Linked';
  readonly fieldName: string;
  readonly alias: string | null;
  readonly selections: ReaderAst<unknown>;
  readonly arguments: Arguments | null;
};

export type ReaderResolverVariant =
  | { readonly kind: 'Eager' }
  // componentName is type + field concatenated
  | { readonly kind: 'Component'; readonly componentName: string };

export type ReaderResolverField = {
  readonly kind: 'Resolver';
  readonly alias: string;
  readonly readerArtifact: ReaderArtifact<any, any>;
  readonly arguments: Arguments | null;
  readonly usedRefetchQueries: number[];
};

export type ReaderRefetchField = {
  readonly kind: 'RefetchField';
  readonly alias: string;
  // TODO this bad modeling. A refetch field cannot have variant: "Component" (I think)
  readonly readerArtifact: ReaderArtifact<any, any>;
  readonly refetchQuery: number;
};

export type ReaderMutationField = {
  readonly kind: 'MutationField';
  readonly alias: string;
  // TODO this bad modeling. A mutation field cannot have variant: "Component" (I think)
  readonly readerArtifact: ReaderArtifact<any, any>;
  readonly refetchQuery: number;
};

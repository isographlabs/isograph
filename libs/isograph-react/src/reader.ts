import { Arguments } from './util';

// TODO this should probably be at least three distinct types, for @component,
// non-@component and refetch resolvers
export type ReaderArtifact<TReadFromStore extends Object, TResolverResult> = {
  kind: 'ReaderArtifact';
  readerAst: ReaderAst<TReadFromStore>;
  resolver: (data: TReadFromStore, runtimeProps: any) => TResolverResult;
  variant: ReaderResolverVariant;
};

export type ReaderAstNode =
  | ReaderScalarField
  | ReaderLinkedField
  | ReaderResolverField
  | ReaderRefetchField
  | ReaderMutationField;

// @ts-ignore
export type ReaderAst<TReadFromStore> = ReaderAstNode[];

export type ReaderScalarField = {
  kind: 'Scalar';
  fieldName: string;
  alias: string | null;
  arguments: Arguments | null;
};
export type ReaderLinkedField = {
  kind: 'Linked';
  fieldName: string;
  alias: string | null;
  selections: ReaderAst<unknown>;
  arguments: Arguments | null;
};

export type ReaderResolverVariant =
  | { kind: 'Eager' }
  // componentName is the component's cacheKey for getRefReaderByName
  // and is the type + field concatenated
  | { kind: 'Component'; componentName: string };

export type ReaderResolverField = {
  kind: 'Resolver';
  alias: string;
  readerArtifact: ReaderArtifact<any, any>;
  arguments: Arguments | null;
  usedRefetchQueries: number[];
};

export type ReaderRefetchField = {
  kind: 'RefetchField';
  alias: string;
  // TODO this bad modeling. A refetch field cannot have variant: "Component" (I think)
  readerArtifact: ReaderArtifact<any, any>;
  refetchQuery: number;
};

export type ReaderMutationField = {
  kind: 'MutationField';
  alias: string;
  // TODO this bad modeling. A mutation field cannot have variant: "Component" (I think)
  readerArtifact: ReaderArtifact<any, any>;
  refetchQuery: number;
};

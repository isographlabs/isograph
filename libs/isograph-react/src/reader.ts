import { Factory } from '@isograph/disposable-types';
import { FragmentReference } from './FragmentReference';
import {
  ComponentOrFieldName,
  DataId,
  IsographEnvironment,
} from './IsographEnvironment';
import {
  RefetchQueryNormalizationArtifact,
  RefetchQueryNormalizationArtifactWrapper,
} from './entrypoint';
import { Arguments } from './util';

export type TopLevelReaderArtifact<
  TReadFromStore extends Object,
  TClientFieldValue,
  TComponentProps extends Record<string, never>,
> =
  | EagerReaderArtifact<TReadFromStore, TClientFieldValue>
  | ComponentReaderArtifact<TReadFromStore, TComponentProps>;

export type EagerReaderArtifact<
  TReadFromStore extends Object,
  TClientFieldValue,
> = {
  readonly kind: 'EagerReaderArtifact';
  readonly readerAst: ReaderAst<TReadFromStore>;
  readonly resolver: (data: TReadFromStore) => TClientFieldValue;
};

export type ComponentReaderArtifact<
  TReadFromStore extends Object,
  TComponentProps extends Record<string, unknown> = Record<string, never>,
> = {
  readonly kind: 'ComponentReaderArtifact';
  readonly componentName: ComponentOrFieldName;
  readonly readerAst: ReaderAst<TReadFromStore>;
  readonly resolver: (
    data: TReadFromStore,
    runtimeProps: TComponentProps,
  ) => React.ReactNode;
};

export type RefetchReaderArtifact = {
  readonly kind: 'RefetchReaderArtifact';
  readonly readerAst: ReaderAst<unknown>;
  readonly resolver: (
    environment: IsographEnvironment,
    artifact: RefetchQueryNormalizationArtifact,
    // TODO type this better
    variables: any,
    // TODO type this better
    filteredVariables: any,
    rootId: DataId,
    readerArtifact: TopLevelReaderArtifact<any, any, any> | null,
    // TODO type this better
    nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[],
  ) => () => void;
};

export type MutationReaderArtifact<TReadFromStore extends Object> = {
  readonly kind: 'MutationReaderArtifact';
  readonly readerAst: ReaderAst<unknown>;
  readonly resolver: (
    environment: IsographEnvironment,
    // TODO type this better
    entrypoint: RefetchQueryNormalizationArtifact,
    readOutData: TReadFromStore,
    // TODO type this better
    filteredVariables: any,
    rootId: DataId,
    readerArtifact: TopLevelReaderArtifact<any, any, any> | null,
    // TODO type this better
    nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[],
  ) => (mutationParams: any) => void;
};

export type ReaderAstNode =
  | ReaderScalarField
  | ReaderLinkedField
  | ReaderResolverField
  | ReaderImperativelyLoadedField;

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

export type ReaderResolverField = {
  readonly kind: 'Resolver';
  readonly alias: string;
  // TODO don't type this as any
  readonly readerArtifact: TopLevelReaderArtifact<any, any, any>;
  readonly arguments: Arguments | null;
  readonly usedRefetchQueries: number[];
};

export type ReaderImperativelyLoadedField = {
  readonly kind: 'ImperativelyLoadedField';
  readonly alias: string;
  readonly refetchReaderArtifact:
    | MutationReaderArtifact<any>
    | RefetchReaderArtifact;
  readonly resolverReaderArtifact: TopLevelReaderArtifact<any, any, any> | null;
  readonly refetchQuery: number;
  readonly name: string;
  readonly usedRefetchQueries: number[];
};

type StableId = string;
/// Why is LoadableField the way it is? Let's work backwards.
///
/// We ultimately need a stable id (for deduplication) and a way to produce a
/// FragmentReference (i.e. a Factory). However, this stable id depends on the
/// arguments that we pass in, hence we get the current form of LoadableField.
///
/// Passing TArgs to the LoadableField should be cheap and do no "actual" work,
/// except to stringify the args or whatnot. Calling the factory can be
/// expensive. For example, doing so will probably trigger a network request.
export type LoadableField<TArgs, TResult> = (
  args: TArgs,
) => [StableId, Factory<FragmentReference<any, TResult>>];

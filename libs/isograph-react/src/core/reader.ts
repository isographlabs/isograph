import { Factory } from '@isograph/disposable-types';
import { FragmentReference, Variables } from './FragmentReference';
import {
  ComponentOrFieldName,
  DataId,
  IsographEnvironment,
} from './IsographEnvironment';
import {
  IsographEntrypoint,
  IsographEntrypointLoader,
  RefetchQueryNormalizationArtifact,
  RefetchQueryNormalizationArtifactWrapper,
} from './entrypoint';
import { Arguments } from './util';

export type TopLevelReaderArtifact<
  TReadFromStore extends { parameters: object; data: object },
  TClientFieldValue,
  TComponentProps extends Record<string, never>,
  TVariables = Variables,
> =
  | EagerReaderArtifact<TReadFromStore, TClientFieldValue, TVariables>
  | ComponentReaderArtifact<TReadFromStore, TComponentProps, TVariables>;

export type EagerReaderArtifact<
  TReadFromStore extends { parameters: object; data: object },
  TClientFieldValue,
  TVariables = Variables,
> = {
  readonly kind: 'EagerReaderArtifact';
  readonly readerAst: ReaderAst<TReadFromStore>;
  readonly resolver: (
    data: ResolverFirstParameter<TReadFromStore, TVariables>,
  ) => TClientFieldValue;
};

export type ComponentReaderArtifact<
  TReadFromStore extends { parameters: object; data: object },
  TComponentProps extends Record<string, unknown> = Record<string, never>,
  TVariables = Variables,
> = {
  readonly kind: 'ComponentReaderArtifact';
  readonly componentName: ComponentOrFieldName;
  readonly readerAst: ReaderAst<TReadFromStore>;
  readonly resolver: (
    data: ResolverFirstParameter<TReadFromStore, TVariables>,
    runtimeProps: TComponentProps,
  ) => React.ReactNode;
};

export type ResolverFirstParameter<
  TReadFromStore extends object,
  TVariables = Variables,
> = {
  readonly data: TReadFromStore;
  readonly parameters: TVariables;
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

export type ReaderAstNode =
  | ReaderScalarField
  | ReaderLinkedField
  | ReaderNonLoadableResolverField
  | ReaderImperativelyLoadedField
  | ReaderLoadableField;

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

export type ReaderNonLoadableResolverField = {
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
  readonly refetchReaderArtifact: RefetchReaderArtifact;
  readonly refetchQuery: number;
  readonly name: string;
};

export type ReaderLoadableField = {
  readonly kind: 'LoadablySelectedField';
  readonly alias: string;

  // To generate a stable id, we need the parent id + the name + the args that
  // we pass to the field, which come from: queryArgs, refetchReaderAst
  // (technically, but in practice that is always "id") and the user-provided args.
  readonly name: string;
  readonly queryArguments: Arguments | null;
  readonly refetchReaderAst: ReaderAst<any>;

  // TODO we should not type these as any
  readonly entrypoint:
    | IsographEntrypoint<any, any>
    | IsographEntrypointLoader<any, any>;
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

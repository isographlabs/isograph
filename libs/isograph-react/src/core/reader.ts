import type { Factory } from '@isograph/disposable-types';
import type { FetchOptions } from './check';
import type {
  IsographEntrypoint,
  IsographEntrypointLoader,
  RefetchQueryNormalizationArtifact,
  RefetchQueryNormalizationArtifactWrapper,
} from './entrypoint';
import type { ExtractParameters, FragmentReference } from './FragmentReference';
import { type UnknownTReadFromStore } from './FragmentReference';
import type {
  ComponentOrFieldName,
  IsographEnvironment,
} from './IsographEnvironment';
import { type StoreLink } from './IsographEnvironment';
import type { Arguments } from './util';

export type TopLevelReaderArtifact<
  TReadFromStore extends UnknownTReadFromStore,
  TClientFieldValue,
  TComponentProps extends Record<PropertyKey, never>,
> =
  | EagerReaderArtifact<TReadFromStore, TClientFieldValue>
  | ComponentReaderArtifact<TReadFromStore, TComponentProps>;

export type EagerReaderArtifact<
  TReadFromStore extends UnknownTReadFromStore,
  TClientFieldValue,
> = {
  readonly kind: 'EagerReaderArtifact';
  readonly fieldName: ComponentOrFieldName;
  readonly readerAst: ReaderAst<TReadFromStore>;
  readonly resolver: (
    data: ResolverFirstParameter<TReadFromStore>,
  ) => TClientFieldValue;
  readonly hasUpdatable: boolean;
};

export type ComponentReaderArtifact<
  TReadFromStore extends UnknownTReadFromStore,
  TComponentProps extends Record<string, unknown> = Record<PropertyKey, never>,
> = {
  readonly kind: 'ComponentReaderArtifact';
  readonly fieldName: ComponentOrFieldName;
  readonly readerAst: ReaderAst<TReadFromStore>;
  readonly resolver: (
    data: ResolverFirstParameter<TReadFromStore>,
    runtimeProps: TComponentProps,
  ) => React.ReactNode;
  readonly hasUpdatable: boolean;
};

export type ResolverFirstParameter<
  TReadFromStore extends UnknownTReadFromStore,
> = Pick<TReadFromStore, 'data' | 'parameters' | 'startUpdate'>;

export type StartUpdate<UpdatableData> = (
  updater: (startUpdateParams: { updatableData: UpdatableData }) => void,
) => void;

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
    rootLink: StoreLink,
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
  | LoadablySelectedField
  | ReaderLinkField;

// @ts-ignore
export type ReaderAst<TReadFromStore> = ReadonlyArray<ReaderAstNode>;

export type ReaderScalarField = {
  readonly kind: 'Scalar';
  readonly fieldName: string;
  readonly alias: string | null;
  readonly arguments: Arguments | null;
  readonly isUpdatable: boolean;
};

export type ReaderLinkField = {
  readonly kind: 'Link';
  readonly alias: string;
};

export type ReaderLinkedField = {
  readonly kind: 'Linked';
  readonly fieldName: string;
  readonly alias: string | null;
  readonly selections: ReaderAst<unknown>;
  readonly arguments: Arguments | null;
  readonly condition:
    | (() => EagerReaderArtifact<
        { data: any; parameters: any; startUpdate?: StartUpdate<any> },
        StoreLink | null | (StoreLink | null)[] | StoreLink[]
      >)
    | null;
  readonly isUpdatable: boolean;
  /**
   * If refetchQueryIndex != null, then the linked field is a client pointer.
   */
  readonly refetchQueryIndex: number | null;
};

export interface ReaderClientPointer extends ReaderLinkedField {
  readonly refetchQueryIndex: number;
}

export type ReaderNonLoadableResolverField = {
  readonly kind: 'Resolver';
  readonly alias: string;
  // TODO don't type this as any
  readonly readerArtifact: () => TopLevelReaderArtifact<any, any, any>;
  readonly arguments: Arguments | null;
  readonly usedRefetchQueries: number[];
};

export type ReaderImperativelyLoadedField = {
  readonly kind: 'ImperativelyLoadedField';
  readonly alias: string;
  readonly refetchReaderArtifact: RefetchReaderArtifact;
  readonly refetchQueryIndex: number;
  readonly name: string;
};

export type LoadablySelectedField = {
  readonly kind: 'LoadablySelectedField';
  readonly alias: string;

  // To generate a stable id, we need the parent id + the name + the args that
  // we pass to the field, which come from: queryArgs, refetchReaderAst
  // (technically, but in practice that is always "id") and the user-provided args.
  readonly name: string;
  readonly queryArguments: Arguments | null;
  readonly refetchReaderAst: ReaderAst<any>;

  // TODO we should not type these as any.
  readonly entrypoint:
    | IsographEntrypoint<any, any, any, any>
    | IsographEntrypointLoader<any, any, any>;
};

export type StableId = string;
/// Why is LoadableField the way it is? Let's work backwards.
///
/// We ultimately need a stable id (for deduplication) and a way to produce a
/// FragmentReference (i.e. a Factory). However, this stable id depends on the
/// arguments that we pass in, hence we get the current form of LoadableField.
///
/// Passing TArgs to the LoadableField should be cheap and do no "actual" work,
/// except to stringify the args or whatnot. Calling the factory can be
/// expensive. For example, doing so will probably trigger a network request.
export type LoadableField<
  TReadFromStore extends UnknownTReadFromStore,
  TResult,
  TArgs = ExtractParameters<TReadFromStore>,
> = (
  args: TArgs | void,
  // Note: fetchOptions is not nullable here because a LoadableField is not a
  // user-facing API. Users should only interact with LoadableFields via APIs
  // like useClientSideDefer. These APIs should have a nullable fetchOptions
  // parameter, and provide a default value ({}) to the LoadableField.
  fetchOptions: FetchOptions<TResult, never>,
) => [StableId, Factory<FragmentReference<TReadFromStore, TResult>>];

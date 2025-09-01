import {
  UNASSIGNED_STATE,
  useUpdatableDisposableState,
} from '@isograph/react-disposable-state';
import { FetchOptions, type RequiredFetchOptions } from '../core/check';
import {
  IsographEntrypoint,
  type ExtractNormalizationAst,
  type ExtractReadFromStore,
  type ExtractResolverResult,
  type NormalizationAst,
  type NormalizationAstLoader,
} from '../core/entrypoint';
import {
  ExtractParameters,
  FragmentReference,
} from '../core/FragmentReference';
import { ROOT_ID } from '../core/IsographEnvironment';
import { maybeMakeNetworkRequest } from '../core/makeNetworkRequest';
import { wrapPromise, wrapResolvedValue } from '../core/PromiseWrapper';
import { useIsographEnvironment } from './IsographEnvironmentProvider';

export type UseImperativeReferenceResult<
  TEntrypoint extends
    | IsographEntrypoint<any, any, NormalizationAst>
    | IsographEntrypoint<any, any, NormalizationAstLoader>,
> = {
  fragmentReference: FragmentReference<
    ExtractReadFromStore<TEntrypoint>,
    ExtractResolverResult<TEntrypoint>
  > | null;
  loadFragmentReference: (
    variables: ExtractParameters<ExtractReadFromStore<TEntrypoint>>,
    ...[
      fetchOptions,
    ]: NormalizationAstLoader extends ExtractNormalizationAst<TEntrypoint>
      ? [fetchOptions: RequiredFetchOptions<ExtractResolverResult<TEntrypoint>>]
      : [fetchOptions?: FetchOptions<ExtractResolverResult<TEntrypoint>>]
  ) => void;
};

export function useImperativeReference<
  TEntrypoint extends
    | IsographEntrypoint<any, any, NormalizationAst>
    | IsographEntrypoint<any, any, NormalizationAstLoader>,
>(entrypoint: TEntrypoint): UseImperativeReferenceResult<TEntrypoint> {
  type TReadFromStore = ExtractReadFromStore<TEntrypoint>;
  type TClientFieldValue = ExtractResolverResult<TEntrypoint>;

  const { state, setState } =
    useUpdatableDisposableState<
      FragmentReference<TReadFromStore, TClientFieldValue>
    >();
  const environment = useIsographEnvironment();
  return {
    fragmentReference: state !== UNASSIGNED_STATE ? state : null,
    loadFragmentReference: (
      variables: ExtractParameters<TReadFromStore>,
      fetchOptions?: FetchOptions<TClientFieldValue>,
    ) => {
      const readerWithRefetchQueries =
        entrypoint.readerWithRefetchQueries.kind ===
        'ReaderWithRefetchQueriesLoader'
          ? wrapPromise(entrypoint.readerWithRefetchQueries.loader())
          : wrapResolvedValue(entrypoint.readerWithRefetchQueries);
      const [networkRequest, disposeNetworkRequest] = maybeMakeNetworkRequest(
        environment,
        entrypoint,
        variables,
        readerWithRefetchQueries,
        fetchOptions ?? null,
      );
      setState([
        {
          kind: 'FragmentReference',
          readerWithRefetchQueries,
          root: { __link: ROOT_ID, __typename: entrypoint.concreteType },
          variables,
          networkRequest,
        },
        () => {
          disposeNetworkRequest();
        },
      ]);
    },
  };
}

// @ts-ignore
function tsTests() {
  let withAst!: IsographEntrypoint<any, 'Foo', NormalizationAst>;
  let withAstLoader!: IsographEntrypoint<any, 'Bar', NormalizationAstLoader>;
  let withAstOrLoader = Math.random() ? withAst : withAstLoader;

  useImperativeReference(withAst).loadFragmentReference({});
  useImperativeReference(withAst).loadFragmentReference(
    {},
    { shouldFetch: 'Yes' },
  );
  useImperativeReference(withAst).loadFragmentReference(
    {},
    { shouldFetch: 'IfNecessary' },
  );

  // @ts-expect-error if there's no ast, require `shouldFetch` to be specified
  useImperativeReference(withAstLoader).loadFragmentReference({});
  useImperativeReference(withAstLoader).loadFragmentReference(
    {},
    { shouldFetch: 'Yes' },
  );
  useImperativeReference(withAstLoader).loadFragmentReference(
    {},
    // @ts-expect-error if there's no ast, `shouldFetch` can't be `IfNecessary`
    { shouldFetch: 'IfNecessary' },
  );

  // if the type is unknown there can be no ast so we should use the same rules
  // @ts-expect-error if there's no ast, require `shouldFetch` to be specified
  useImperativeReference(withAstOrLoader).loadFragmentReference({});
  useImperativeReference(withAstOrLoader).loadFragmentReference(
    {},
    { shouldFetch: 'Yes' },
  );
  useImperativeReference(withAstOrLoader) satisfies {
    readonly fragmentReference: FragmentReference<any, 'Foo' | 'Bar'> | null;
  };
  useImperativeReference(withAstOrLoader).loadFragmentReference(
    {},
    // @ts-expect-error if there's no ast, `shouldFetch` can't be `IfNecessary`
    { shouldFetch: 'IfNecessary' },
  );
}

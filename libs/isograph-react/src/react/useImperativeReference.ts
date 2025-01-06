import {
  UnassignedState,
  useUpdatableDisposableState,
} from '@isograph/react-disposable-state';
import {
  IsographEntrypoint,
  type NormalizationAst,
  type NormalizationAstLoader,
} from '../core/entrypoint';
import {
  FragmentReference,
  ExtractParameters,
} from '../core/FragmentReference';
import { useIsographEnvironment } from './IsographEnvironmentProvider';
import { ROOT_ID } from '../core/IsographEnvironment';
import { maybeMakeNetworkRequest } from '../core/makeNetworkRequest';
import { wrapResolvedValue } from '../core/PromiseWrapper';
import { FetchOptions, type RequiredFetchOptions } from '../core/check';

// TODO rename this to useImperativelyLoadedEntrypoint
type UseImperativeReferenceResult<
  TReadFromStore extends { parameters: object; data: object },
  TClientFieldValue,
> = {
  fragmentReference:
    | FragmentReference<TReadFromStore, TClientFieldValue>
    | UnassignedState;
  loadFragmentReference: (
    variables: ExtractParameters<TReadFromStore>,
    fetchOptions?: FetchOptions<TClientFieldValue>,
  ) => void;
};

type RequiredUseImperativeReferenceResult<
  TReadFromStore extends { parameters: object; data: object },
  TClientFieldValue,
> = {
  fragmentReference:
    | FragmentReference<TReadFromStore, TClientFieldValue>
    | UnassignedState;
  loadFragmentReference: (
    variables: ExtractParameters<TReadFromStore>,
    fetchOptions: RequiredFetchOptions<TClientFieldValue>,
  ) => void;
};

export function useImperativeReference<
  TReadFromStore extends { parameters: object; data: object },
  TClientFieldValue,
>(
  entrypoint: IsographEntrypoint<
    TReadFromStore,
    TClientFieldValue,
    NormalizationAstLoader
  >,
): RequiredUseImperativeReferenceResult<TReadFromStore, TClientFieldValue>;
export function useImperativeReference<
  TReadFromStore extends { parameters: object; data: object },
  TClientFieldValue,
>(
  entrypoint: IsographEntrypoint<
    TReadFromStore,
    TClientFieldValue,
    NormalizationAst
  >,
): UseImperativeReferenceResult<TReadFromStore, TClientFieldValue>;
export function useImperativeReference<
  TReadFromStore extends { parameters: object; data: object },
  TClientFieldValue,
>(
  entrypoint: IsographEntrypoint<TReadFromStore, TClientFieldValue>,
): UseImperativeReferenceResult<TReadFromStore, TClientFieldValue> {
  const { state, setState } =
    useUpdatableDisposableState<
      FragmentReference<TReadFromStore, TClientFieldValue>
    >();
  const environment = useIsographEnvironment();
  return {
    fragmentReference: state,
    loadFragmentReference: (
      variables: ExtractParameters<TReadFromStore>,
      fetchOptions?: FetchOptions<TClientFieldValue>,
    ) => {
      const [networkRequest, disposeNetworkRequest] = maybeMakeNetworkRequest(
        environment,
        entrypoint as IsographEntrypoint<any, any, NormalizationAst>,
        variables,
        fetchOptions,
      );
      setState([
        {
          kind: 'FragmentReference',
          readerWithRefetchQueries: wrapResolvedValue({
            kind: 'ReaderWithRefetchQueries',
            readerArtifact: entrypoint.readerWithRefetchQueries.readerArtifact,
            nestedRefetchQueries:
              entrypoint.readerWithRefetchQueries.nestedRefetchQueries,
          }),
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

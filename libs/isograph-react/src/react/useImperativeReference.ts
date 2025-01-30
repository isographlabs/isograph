import {
  UNASSIGNED_STATE,
  useUpdatableDisposableState,
} from '@isograph/react-disposable-state';
import { FetchOptions, type RequiredFetchOptions } from '../core/check';
import {
  IsographEntrypoint,
  type NormalizationAst,
  type NormalizationAstLoader,
} from '../core/entrypoint';
import {
  ExtractParameters,
  FragmentReference,
  type UnknownTReadFromStore,
} from '../core/FragmentReference';
import { ROOT_ID } from '../core/IsographEnvironment';
import { maybeMakeNetworkRequest } from '../core/makeNetworkRequest';
import { wrapResolvedValue } from '../core/PromiseWrapper';
import { useIsographEnvironment } from './IsographEnvironmentProvider';

type UseImperativeReferenceResult<
  TReadFromStore extends UnknownTReadFromStore,
  TClientFieldValue,
  TNormalizationAst extends NormalizationAst | NormalizationAstLoader,
> = {
  fragmentReference: FragmentReference<
    TReadFromStore,
    TClientFieldValue
  > | null;
  loadFragmentReference: (
    variables: ExtractParameters<TReadFromStore>,
    ...[fetchOptions]: NormalizationAstLoader extends TNormalizationAst
      ? [fetchOptions: RequiredFetchOptions<TClientFieldValue>]
      : [fetchOptions?: FetchOptions<TClientFieldValue>]
  ) => void;
};

export function useImperativeReference<
  TReadFromStore extends UnknownTReadFromStore,
  TClientFieldValue,
  TNormalizationAst extends NormalizationAst | NormalizationAstLoader,
>(
  entrypoint: IsographEntrypoint<
    TReadFromStore,
    TClientFieldValue,
    TNormalizationAst
  >,
): UseImperativeReferenceResult<
  TReadFromStore,
  TClientFieldValue,
  TNormalizationAst
> {
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
      const [networkRequest, disposeNetworkRequest] = maybeMakeNetworkRequest(
        environment,
        entrypoint,
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

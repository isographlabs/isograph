import {
  UnassignedState,
  useUpdatableDisposableState,
} from '@isograph/react-disposable-state';
import { IsographEntrypoint } from '../core/entrypoint';
import {
  FragmentReference,
  ExtractParameters,
} from '../core/FragmentReference';
import { useIsographEnvironment } from './IsographEnvironmentProvider';
import { ROOT_ID } from '../core/IsographEnvironment';
import { maybeMakeNetworkRequest } from '../core/makeNetworkRequest';
import { wrapResolvedValue } from '../core/PromiseWrapper';
import { FetchOptions } from '../core/check';
import type { StartUpdate } from '../core/reader';

// TODO rename this to useImperativelyLoadedEntrypoint

export function useImperativeReference<
  TReadFromStore extends {
    parameters: object;
    data: object;
    startUpdate?: StartUpdate<object>;
  },
  TClientFieldValue,
>(
  entrypoint: IsographEntrypoint<TReadFromStore, TClientFieldValue>,
): {
  fragmentReference:
    | FragmentReference<TReadFromStore, TClientFieldValue>
    | UnassignedState;
  loadFragmentReference: (
    variables: ExtractParameters<TReadFromStore>,
    fetchOptions?: FetchOptions<TClientFieldValue>,
  ) => void;
} {
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

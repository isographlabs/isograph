import {
  UnassignedState,
  useUpdatableDisposableState,
} from '@isograph/react-disposable-state';
import { IsographEntrypoint } from '../core/entrypoint';
import { FragmentReference, Variables } from '../core/FragmentReference';
import { useIsographEnvironment } from './IsographEnvironmentProvider';
import { ROOT_ID } from '../core/IsographEnvironment';
import { makeNetworkRequest } from '../core/makeNetworkRequest';
import { wrapResolvedValue } from '../core/PromiseWrapper';

// TODO rename this to useImperativelyLoadedEntrypoint

export function useImperativeReference<
  TReadFromStore extends { parameters: object; data: object },
  TClientFieldValue,
>(
  entrypoint: IsographEntrypoint<TReadFromStore, TClientFieldValue>,
): {
  fragmentReference:
    | FragmentReference<TReadFromStore, TClientFieldValue>
    | UnassignedState;
  loadFragmentReference: (variables: Variables) => void;
} {
  const { state, setState } =
    useUpdatableDisposableState<
      FragmentReference<TReadFromStore, TClientFieldValue>
    >();
  const environment = useIsographEnvironment();
  return {
    fragmentReference: state,
    loadFragmentReference: (variables: Variables) => {
      const [networkRequest, disposeNetworkRequest] = makeNetworkRequest(
        environment,
        entrypoint,
        variables,
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
          root: ROOT_ID,
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

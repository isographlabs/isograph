import {
  UnassignedState,
  useUpdatableDisposableState,
} from '@isograph/react-disposable-state';
import { IsographEntrypoint } from '../core/entrypoint';
import { FragmentReference, Variables } from '../core/FragmentReference';
import { useIsographEnvironment } from './IsographEnvironmentProvider';
import { makeNetworkRequest } from '../core/cache';
import { ROOT_ID } from '../core/IsographEnvironment';

export function useImperativeReference<
  TReadFromStore extends Object,
  TClientFieldValue,
>(
  entrypoint: IsographEntrypoint<TReadFromStore, TClientFieldValue>,
): {
  fragmentReference:
    | FragmentReference<TReadFromStore, TClientFieldValue>
    | UnassignedState;
  loadfragmentReference: (variables: Variables) => void;
} {
  const { state, setState } =
    useUpdatableDisposableState<
      FragmentReference<TReadFromStore, TClientFieldValue>
    >();
  const environment = useIsographEnvironment();
  return {
    fragmentReference: state,
    loadfragmentReference: (variables: Variables) => {
      const [_networkRequest, disposeNetworkRequest] = makeNetworkRequest(
        environment,
        entrypoint,
        variables,
      );
      setState([
        {
          kind: 'FragmentReference',
          readerArtifact: entrypoint.readerArtifact,
          root: ROOT_ID,
          variables,
          nestedRefetchQueries: entrypoint.nestedRefetchQueries,
        },
        () => {
          disposeNetworkRequest();
        },
      ]);
    },
  };
}

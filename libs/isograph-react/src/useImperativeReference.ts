import {
  UnassignedState,
  useUpdatableDisposableState,
} from '@isograph/react-disposable-state';
import { IsographEntrypoint } from './entrypoint';
import { FragmentReference } from './FragmentReference';
import { useIsographEnvironment } from './IsographEnvironmentProvider';
import { makeNetworkRequest } from './cache';
import { ROOT_ID } from './IsographEnvironment';

export function useImperativeReference<
  TReadFromStore extends Object,
  TClientFieldValue,
>(
  entrypoint: IsographEntrypoint<TReadFromStore, TClientFieldValue>,
): {
  queryReference:
    | FragmentReference<TReadFromStore, TClientFieldValue>
    | UnassignedState;
  loadQueryReference: (variables: { [index: string]: string }) => void;
} {
  const { state, setState } =
    useUpdatableDisposableState<
      FragmentReference<TReadFromStore, TClientFieldValue>
    >();
  const environment = useIsographEnvironment();
  return {
    queryReference: state,
    loadQueryReference: (variables: { [index: string]: string }) => {
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

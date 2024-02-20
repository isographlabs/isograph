import {
  UnassignedState,
  useUpdatableDisposableState,
} from '@isograph/react-disposable-state';
import {
  type IsographEntrypoint,
  type FragmentReference,
  ExtractReadFromStore,
  ExtractResolverProps,
  ExtractResolverResult,
  ROOT_ID,
  useIsographEnvironment,
  makeNetworkRequest,
} from './index';

export function useImperativeReference<
  TEntrypoint extends IsographEntrypoint<any, any, any>,
>(
  entrypoint: TEntrypoint,
): {
  queryReference:
    | FragmentReference<
        ExtractReadFromStore<TEntrypoint>,
        ExtractResolverProps<TEntrypoint>,
        ExtractResolverResult<TEntrypoint>
      >
    | UnassignedState;
  loadQueryReference: (variables: { [index: string]: string }) => void;
} {
  const { state, setState } =
    useUpdatableDisposableState<
      FragmentReference<
        ExtractReadFromStore<TEntrypoint>,
        ExtractResolverProps<TEntrypoint>,
        ExtractResolverResult<TEntrypoint>
      >
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

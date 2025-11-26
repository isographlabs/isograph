import { getOrCreateCachedComponent } from '../core/componentCache';
import {
  FragmentReference,
  type UnknownTReadFromStore,
} from '../core/FragmentReference';
import {
  getPromiseState,
  PromiseWrapper,
  readPromise,
} from '../core/PromiseWrapper';
import {
  getNetworkRequestOptionsWithDefaults,
  NetworkRequestReaderOptions,
} from '../core/read';
import { getOrCreateCachedStartUpdate } from '../core/startUpdate';
import { useIsographEnvironment } from '../react/IsographEnvironmentProvider';
import { useReadAndSubscribe } from './useReadAndSubscribe';

export function useResult<
  TReadFromStore extends UnknownTReadFromStore,
  TClientFieldValue,
>(
  fragmentReference: FragmentReference<TReadFromStore, TClientFieldValue>,
  partialNetworkRequestOptions?: Partial<NetworkRequestReaderOptions> | void,
): TClientFieldValue {
  const environment = useIsographEnvironment();
  const networkRequestOptions = getNetworkRequestOptionsWithDefaults(
    partialNetworkRequestOptions,
  );

  switch (fragmentReference.readerArtifactKind) {
    case 'ComponentReaderArtifact': {
      // @ts-expect-error
      return getOrCreateCachedComponent(
        environment,
        fragmentReference,
        networkRequestOptions,
      );
    }
    case 'EagerReaderArtifact': {
      maybeUnwrapNetworkRequest(
        fragmentReference.networkRequest,
        networkRequestOptions,
      );
      const readerWithRefetchQueries = readPromise(
        fragmentReference.readerWithRefetchQueries,
      );
      const data = useReadAndSubscribe(
        fragmentReference,
        networkRequestOptions,
        readerWithRefetchQueries.readerArtifact.readerAst,
      );
      const param = {
        data: data,
        parameters: fragmentReference.variables,
        ...(readerWithRefetchQueries.readerArtifact.hasUpdatable
          ? {
              startUpdate: getOrCreateCachedStartUpdate(
                environment,
                fragmentReference,
                networkRequestOptions,
              ),
            }
          : undefined),
      };
      // @ts-expect-error
      return readerWithRefetchQueries.readerArtifact.resolver(param);
    }
  }
}

export function maybeUnwrapNetworkRequest(
  networkRequest: PromiseWrapper<void, any>,
  networkRequestOptions: NetworkRequestReaderOptions,
) {
  const state = getPromiseState(networkRequest);
  if (state.kind === 'Err' && networkRequestOptions.throwOnNetworkError) {
    throw state.error;
  } else if (
    state.kind === 'Pending' &&
    networkRequestOptions.suspendIfInFlight
  ) {
    throw state.promise;
  }
}

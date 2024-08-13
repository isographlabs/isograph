import { useIsographEnvironment } from '../react/IsographEnvironmentProvider';
import { FragmentReference } from '../core/FragmentReference';
import { getOrCreateCachedComponent } from '../core/componentCache';
import { useReadAndSubscribe } from './useReadAndSubscribe';
import {
  getNetworkRequestOptionsWithDefaults,
  NetworkRequestReaderOptions,
} from '../core/read';
import { getPromiseState, PromiseWrapper } from '../core/PromiseWrapper';

export function useResult<TReadFromStore extends Object, TClientFieldValue>(
  fragmentReference: FragmentReference<TReadFromStore, TClientFieldValue>,
  partialNetworkRequestOptions?: Partial<NetworkRequestReaderOptions> | void,
): TClientFieldValue {
  const environment = useIsographEnvironment();
  const networkRequestOptions = getNetworkRequestOptionsWithDefaults(
    partialNetworkRequestOptions,
  );

  maybeUnwrapNetworkRequest(
    fragmentReference.networkRequest,
    networkRequestOptions,
  );

  switch (fragmentReference.readerWithRefetchQueries.readerArtifact.kind) {
    case 'ComponentReaderArtifact': {
      // @ts-expect-error
      return getOrCreateCachedComponent(
        environment,
        fragmentReference.readerWithRefetchQueries.readerArtifact.componentName,
        fragmentReference,
        networkRequestOptions,
      );
    }
    case 'EagerReaderArtifact': {
      const data = useReadAndSubscribe(
        fragmentReference,
        networkRequestOptions,
      );
      return fragmentReference.readerWithRefetchQueries.readerArtifact.resolver(
        data,
      );
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

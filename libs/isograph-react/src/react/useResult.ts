import { useIsographEnvironment } from '../react/IsographEnvironmentProvider';
import { FragmentReference } from '../core/FragmentReference';
import { getOrCreateCachedComponent } from '../core/componentCache';
import { useReadAndSubscribe } from './useReadAndSubscribe';
import { NetworkRequestReaderOptions } from '../core/read';
import { getPromiseState, PromiseWrapper } from '../core/PromiseWrapper';

export function useResult<TReadFromStore extends Object, TClientFieldValue>(
  fragmentReference: FragmentReference<TReadFromStore, TClientFieldValue>,
  networkRequestOptions: NetworkRequestReaderOptions,
): TClientFieldValue {
  const environment = useIsographEnvironment();

  maybeUnwrapNetworkRequest(
    fragmentReference.networkRequest,
    networkRequestOptions,
  );

  switch (fragmentReference.readerArtifact.kind) {
    case 'ComponentReaderArtifact': {
      // @ts-expect-error
      return getOrCreateCachedComponent(
        environment,
        fragmentReference.readerArtifact.componentName,
        fragmentReference,
        networkRequestOptions,
      );
    }
    case 'EagerReaderArtifact': {
      const data = useReadAndSubscribe(
        environment,
        fragmentReference,
        networkRequestOptions,
      );
      return fragmentReference.readerArtifact.resolver(data);
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

import { getOrCreateCachedComponent } from '../core/componentCache';
import { FragmentReference } from '../core/FragmentReference';
import {
  getPromiseState,
  PromiseWrapper,
  readPromise,
} from '../core/PromiseWrapper';
import {
  getNetworkRequestOptionsWithDefaults,
  NetworkRequestReaderOptions,
} from '../core/read';
import type { StartUpdate } from '../core/reader';
import { startUpdate } from '../core/startUpdate';
import { useIsographEnvironment } from '../react/IsographEnvironmentProvider';
import { useReadAndSubscribe } from './useReadAndSubscribe';

export function useResult(
  fragmentReference: null | undefined,
  partialNetworkRequestOptions?: Partial<NetworkRequestReaderOptions> | void,
): null;

export function useResult<
  TReadFromStore extends {
    parameters: object;
    data: object;
    startUpdate?: StartUpdate<object>;
  },
  TClientFieldValue,
>(
  fragmentReference: FragmentReference<TReadFromStore, TClientFieldValue>,
  partialNetworkRequestOptions?: Partial<NetworkRequestReaderOptions> | void,
): TClientFieldValue;

export function useResult<
  TReadFromStore extends {
    parameters: object;
    data: object;
    startUpdate?: StartUpdate<object>;
  },
  TClientFieldValue,
>(
  fragmentReference:
    | FragmentReference<TReadFromStore, TClientFieldValue>
    | null
    | undefined,
  partialNetworkRequestOptions?: Partial<NetworkRequestReaderOptions> | void,
): TClientFieldValue | null;

export function useResult<
  TReadFromStore extends {
    parameters: object;
    data: object;
    startUpdate?: StartUpdate<object>;
  },
  TClientFieldValue,
>(
  fragmentReference:
    | FragmentReference<TReadFromStore, TClientFieldValue>
    | null
    | undefined,
  partialNetworkRequestOptions?: Partial<NetworkRequestReaderOptions> | void,
): TClientFieldValue | null {
  if (fragmentReference == null) {
    return null;
  }

  const environment = useIsographEnvironment();
  const networkRequestOptions = getNetworkRequestOptionsWithDefaults(
    partialNetworkRequestOptions,
  );

  maybeUnwrapNetworkRequest(
    fragmentReference.networkRequest,
    networkRequestOptions,
  );

  const readerWithRefetchQueries = readPromise(
    fragmentReference.readerWithRefetchQueries,
  );

  switch (readerWithRefetchQueries.readerArtifact.kind) {
    case 'ComponentReaderArtifact': {
      // @ts-expect-error
      return getOrCreateCachedComponent(
        environment,
        readerWithRefetchQueries.readerArtifact.componentName,
        fragmentReference,
        networkRequestOptions,
      );
    }
    case 'EagerReaderArtifact': {
      const data = useReadAndSubscribe(
        fragmentReference,
        networkRequestOptions,
        readerWithRefetchQueries.readerArtifact.readerAst,
      );
      const param = {
        data: data,
        parameters: fragmentReference.variables,
        startUpdate: readerWithRefetchQueries.readerArtifact.hasUpdatable
          ? startUpdate(environment, data)
          : undefined,
      };
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

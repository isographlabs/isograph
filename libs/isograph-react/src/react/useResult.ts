import { getOrCreateCachedComponent } from '../core/componentCache';
import type {
  FragmentReference,
  UnknownTReadFromStore,
} from '../core/FragmentReference';
import { readPromise } from '../core/PromiseWrapper';
import {
  type NetworkRequestReaderOptions,
  getNetworkRequestOptionsWithDefaults,
} from '../core/read';
import { getOrCreateCachedStartUpdate } from '../core/startUpdate';
import { useIsographEnvironment } from './IsographEnvironmentProvider';
import { maybeUnwrapNetworkRequest } from './maybeUnwrapNetworkRequest';
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
      return readerWithRefetchQueries.readerArtifact.resolver({
        firstParameter: param,
      });
    }
  }
}

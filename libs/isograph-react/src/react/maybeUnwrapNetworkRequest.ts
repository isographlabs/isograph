import { type PromiseWrapper, getPromiseState } from '../core/PromiseWrapper';
import type { NetworkRequestReaderOptions } from '../core/read';

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

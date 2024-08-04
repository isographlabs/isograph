import { PromiseWrapper, readPromise } from './PromiseWrapper';

export type NetworkRequestReference = {
  kind: 'NetworkRequestReference';
  promise: PromiseWrapper<void, any>;
};

export function readNetworkRequestReference(
  networkRequestReference: NetworkRequestReference,
): void {
  return readPromise(networkRequestReference.promise);
}

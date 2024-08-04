import { NetworkRequestReference } from '../core/NetworkRequestReference';
import { getPromiseState, readPromise } from '../core/PromiseWrapper';

export function NetworkRequestReader({
  networkRequestReference,
  children,
}: {
  networkRequestReference: NetworkRequestReference;
  children: React.ReactNode;
}) {
  readPromise(networkRequestReference.promise);
  return children;
}

export function NetworkErrorReader({
  networkRequestReference,
  children,
}: {
  networkRequestReference: NetworkRequestReference;
  children: React.ReactNode;
}) {
  const state = getPromiseState(networkRequestReference.promise);
  if (state.kind === 'Err') {
    throw state.error;
  }
  return children;
}

import { ItemCleanupPair } from '@isograph/isograph-disposable-types';
import { FragmentReference } from '../FragmentReference';
import { useIsographEnvironment } from '../IsographEnvironmentProvider';
import { getOrCreateCache } from '../cache';
import { useLazyDisposableState } from '@isograph/react-disposable-state';
import { DataId } from '../IsographEnvironment';

// TODO allow the user to pass props somehow
export function useClientSideDefer<TValue>([id, loader]: [
  DataId,
  () => ItemCleanupPair<FragmentReference<any, TValue>>,
]) {
  const environment = useIsographEnvironment();
  const cache = getOrCreateCache(environment, id, loader);

  const fragmentReference = useLazyDisposableState(cache).state;

  return fragmentReference;
}

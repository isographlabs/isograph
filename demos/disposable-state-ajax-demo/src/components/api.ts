import {
  Factory,
  ItemCleanupPair,
  ParentCache,
} from '@isograph/react-disposable-state';
import { PromiseWrapper, wrapPromise } from './PromiseWrapper';

const cache: { [index: string]: ParentCache<any> } = {};

function getOrCreateCache<T>(
  index: string,
  factory: Factory<T>,
): ParentCache<T> {
  if (cache[index] == null) {
    cache[index] = new ParentCache(factory);
  }

  return cache[index];
}

export function getOrCreateCacheForUrl<T extends object>(
  url: string,
): ParentCache<PromiseWrapper<T>> {
  const factory: Factory<PromiseWrapper<T>> = () => makeNetworkRequest<T>(url);
  return getOrCreateCache<PromiseWrapper<T>>(url, factory);
}

export function makeNetworkRequest<T extends object>(
  url: string,
): ItemCleanupPair<PromiseWrapper<T>> {
  let promise: Promise<T> = fetch(url).then((response) => response.json());

  const wrapper = wrapPromise(promise);

  const response: ItemCleanupPair<PromiseWrapper<T>> = [wrapper, () => {}];
  return response;
}

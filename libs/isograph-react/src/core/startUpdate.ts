import {
  stableIdForFragmentReference,
  type ExtractStartUpdate,
  type FragmentReference,
  type UnknownTReadFromStore,
} from './FragmentReference';
import type { IsographEnvironment } from './IsographEnvironment';

export function getOrCreateCachedStartUpdate<
  TReadFromStore extends UnknownTReadFromStore,
>(
  environment: IsographEnvironment,
  fragmentReference: FragmentReference<TReadFromStore, any>,
  eagerResolverName: string,
): ExtractStartUpdate<TReadFromStore> {
  return (environment.eagerReaderCache[
    stableIdForFragmentReference(fragmentReference, eagerResolverName)
  ] ??= createStartUpdate(environment, fragmentReference));
}

export function createStartUpdate<TReadFromStore extends UnknownTReadFromStore>(
  _environment: IsographEnvironment,
  _fragmentReference: FragmentReference<TReadFromStore, any>,
): ExtractStartUpdate<TReadFromStore> {
  return (_updater) => {
    // TODO start update
  };
}

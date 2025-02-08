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
  const cachedStartUpdateByResolver = environment.fieldCache;

  const cacheEntry = (cachedStartUpdateByResolver[
    stableIdForFragmentReference(fragmentReference, eagerResolverName)
  ] ??= {
    kind: 'EagerReader',
    startUpdate: undefined,
  });

  switch (cacheEntry.kind) {
    case 'EagerReader': {
      return (cacheEntry.startUpdate ??= createStartUpdate(
        environment,
        fragmentReference,
      ));
    }
    case 'Component': {
      throw new Error(
        'Called getOrCreateCachedStartUpdate on a component. ' +
          'This is indicative of a bug in Isograph.',
      );
    }
  }
}

export function createStartUpdate<TReadFromStore extends UnknownTReadFromStore>(
  _environment: IsographEnvironment,
  _fragmentReference: FragmentReference<TReadFromStore, any>,
): ExtractStartUpdate<TReadFromStore> {
  return (_updater) => {
    // TODO start update
  };
}
